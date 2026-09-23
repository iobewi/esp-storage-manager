#![no_std]

//! Single-owner flash/NVS coordination for bare-metal Rust on ESP MCUs.
//!
//! `esp_storage::FlashStorage::new()` represents the physical flash device and
//! must not be independently constructed by every subsystem that wants to use
//! NVS or raw flash. This crate owns that one instance, keeps one cached
//! `esp_nvs::Nvs` view of a caller-supplied NVS partition, and provides
//! bounded exclusive raw-flash access for non-overlapping partitions such as
//! OTA slots.
//!
//! The crate deliberately knows nothing about application keys, OTA state,
//! Wi-Fi credentials, TLS material, or any other domain model.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::ptr::NonNull;

use embedded_storage::nor_flash::{ErrorType, MultiwriteNorFlash, NorFlash, ReadNorFlash};
use esp_hal::peripherals::FLASH;
use esp_nvs::error::Error as NvsError;
use esp_nvs::platform::Crc;
use esp_nvs::{Get, Nvs, Set};
use log::warn;
use static_cell::StaticCell;

pub use esp_nvs::Key;
pub use esp_storage::FlashStorage;

/// The flash range containing one ESP-IDF-compatible NVS partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvsPartition {
    pub offset: usize,
    pub size: usize,
}

impl NvsPartition {
    pub const fn new(offset: usize, size: usize) -> Self {
        Self { offset, size }
    }
}

/// Why a persistent mutation could not be completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    /// The NVS partition could not be opened.
    Unavailable,
    /// NVS returned an error while writing or deleting.
    Write,
}

/// Cheap copyable handle to the one physical [`FlashStorage`].
///
/// The pointee is owned by the process-wide static cell below. Access is only
/// performed through a mutable [`StorageManager`], so the cached NVS handle
/// and raw-flash users cannot dereference it concurrently through this API.
#[derive(Clone, Copy)]
struct SharedFlash(NonNull<FlashStorage<'static>>);

// SAFETY: the pointer targets `FLASH_STORAGE`, whose access is serialized by
// `&mut StorageManager`. See `StorageManager::with_raw_flash`.
unsafe impl Send for SharedFlash {}

impl SharedFlash {
    fn flash(&mut self) -> &mut FlashStorage<'static> {
        // SAFETY: see the `Send` implementation and `StorageManager` invariant.
        unsafe { self.0.as_mut() }
    }
}

static FLASH_STORAGE: StaticCell<FlashStorage<'static>> = StaticCell::new();

impl ErrorType for SharedFlash {
    type Error = <FlashStorage<'static> as ErrorType>::Error;
}

impl ReadNorFlash for SharedFlash {
    const READ_SIZE: usize = <FlashStorage<'static> as ReadNorFlash>::READ_SIZE;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.flash().read(offset, bytes)
    }

    fn capacity(&self) -> usize {
        // SAFETY: immutable metadata access to the same process-wide object.
        unsafe { self.0.as_ref() }.capacity()
    }
}

impl NorFlash for SharedFlash {
    const WRITE_SIZE: usize = <FlashStorage<'static> as NorFlash>::WRITE_SIZE;
    const ERASE_SIZE: usize = <FlashStorage<'static> as NorFlash>::ERASE_SIZE;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.flash().erase(from, to)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.flash().write(offset, bytes)
    }
}

impl MultiwriteNorFlash for SharedFlash {}

impl Crc for SharedFlash {
    fn crc32(init: u32, data: &[u8]) -> u32 {
        <FlashStorage<'static> as Crc>::crc32(init, data)
    }
}

/// Owns the single raw flash instance and one cached NVS view.
///
/// Construct exactly once in a firmware image. The underlying `StaticCell`
/// intentionally makes a second construction fail rather than create two
/// independent owners of the same physical flash.
pub struct StorageManager {
    flash: SharedFlash,
    nvs: Option<Nvs<SharedFlash>>,
    partition: NvsPartition,
    healthy: bool,
}

impl StorageManager {
    pub fn new(flash: FLASH<'static>, partition: NvsPartition) -> Self {
        let flash = FLASH_STORAGE.init(FlashStorage::new(flash));
        Self {
            flash: SharedFlash(NonNull::from(flash)),
            nvs: None,
            partition,
            healthy: true,
        }
    }

    /// Runs `f` with exclusive access to the physical flash.
    ///
    /// Keeping the cached `Nvs` alive is sound as long as raw users do not
    /// modify the NVS partition itself. The caller owns that partitioning
    /// invariant; this method cannot infer which raw offsets `f` touches.
    pub fn with_raw_flash<R>(&mut self, f: impl FnOnce(&mut FlashStorage<'static>) -> R) -> R {
        // SAFETY: `&mut self` excludes all NVS operations through this manager
        // for the entire closure call. `Nvs` stores `SharedFlash`, not a live
        // Rust reference to the pointee.
        f(unsafe { &mut *self.flash.0.as_ptr() })
    }

    fn nvs(&mut self) -> Option<&mut Nvs<SharedFlash>> {
        if self.nvs.is_none() {
            match Nvs::new(self.partition.offset, self.partition.size, self.flash) {
                Ok(nvs) => self.nvs = Some(nvs),
                Err(e) => {
                    warn!("NVS unavailable: {e:?}");
                    return None;
                }
            }
        }
        self.nvs.as_mut()
    }

    pub fn get_string(&mut self, namespace: &Key, key: &Key) -> Option<String> {
        self.get(namespace, key)
    }

    pub fn set_string(&mut self, namespace: &Key, key: &Key, value: &str) -> Result<(), StorageError> {
        self.set(namespace, key, value)
    }

    pub fn get_u8(&mut self, namespace: &Key, key: &Key) -> Option<u8> {
        self.get(namespace, key)
    }

    pub fn set_u8(&mut self, namespace: &Key, key: &Key, value: u8) -> Result<(), StorageError> {
        self.set(namespace, key, value)
    }

    pub fn get_bool(&mut self, namespace: &Key, key: &Key) -> Option<bool> {
        self.get(namespace, key)
    }

    pub fn set_bool(&mut self, namespace: &Key, key: &Key, value: bool) -> Result<(), StorageError> {
        self.set(namespace, key, value)
    }

    pub fn get_u16(&mut self, namespace: &Key, key: &Key) -> Option<u16> {
        self.get(namespace, key)
    }

    pub fn set_u16(&mut self, namespace: &Key, key: &Key, value: u16) -> Result<(), StorageError> {
        self.set(namespace, key, value)
    }

    pub fn get_u32(&mut self, namespace: &Key, key: &Key) -> Option<u32> {
        self.get(namespace, key)
    }

    pub fn set_u32(&mut self, namespace: &Key, key: &Key, value: u32) -> Result<(), StorageError> {
        self.set(namespace, key, value)
    }

    /// Returns an owned snapshot of every decodable `(namespace, key)` pair.
    pub fn keys(&mut self) -> Vec<(Key, Key)> {
        let Some(nvs) = self.nvs() else {
            return Vec::new();
        };
        nvs.keys().filter_map(Result::ok).collect()
    }

    /// Deletes a key. Missing namespaces/keys are treated as success.
    pub fn delete(&mut self, namespace: &Key, key: &Key) -> Result<(), StorageError> {
        let nvs = self.nvs().ok_or(StorageError::Unavailable)?;
        match nvs.delete(namespace, key) {
            Ok(()) | Err(NvsError::NamespaceNotFound | NvsError::KeyNotFound) => Ok(()),
            Err(e) => {
                warn!("Failed to delete {}: {e:?}", key.as_str());
                self.nvs = None;
                self.healthy = false;
                Err(StorageError::Write)
            }
        }
    }

    /// Write/read/delete canary check for the configured NVS partition.
    pub fn self_check(&mut self, namespace: &Key, key: &Key) -> bool {
        const VALUE: u8 = 0xA5;
        let written = self.set_u8(namespace, key, VALUE).is_ok();
        let read_back = self.get_u8(namespace, key) == Some(VALUE);
        let erased = self.delete(namespace, key).is_ok();
        let ok = written && read_back && erased;
        self.healthy = ok;
        ok
    }

    /// Last known NVS health. This is RAM-only and performs no flash access.
    pub fn is_healthy(&self) -> bool {
        self.healthy
    }

    fn get<T>(&mut self, namespace: &Key, key: &Key) -> Option<T>
    where
        Nvs<SharedFlash>: Get<T>,
    {
        let nvs = self.nvs()?;
        match nvs.get(namespace, key) {
            Ok(value) => Some(value),
            Err(NvsError::NamespaceNotFound | NvsError::KeyNotFound) => None,
            Err(e) => {
                warn!("Failed to read {}: {e:?}", key.as_str());
                self.nvs = None;
                self.healthy = false;
                None
            }
        }
    }

    fn set<T>(&mut self, namespace: &Key, key: &Key, value: T) -> Result<(), StorageError>
    where
        Nvs<SharedFlash>: Set<T>,
    {
        let nvs = self.nvs().ok_or(StorageError::Unavailable)?;
        nvs.set(namespace, key, value).map_err(|e| {
            warn!("Failed to save {}: {e:?}", key.as_str());
            self.nvs = None;
            self.healthy = false;
            StorageError::Write
        })
    }
}
