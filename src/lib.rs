#![no_std]

//! Shared ESP storage hardware backend.
//!
//! This crate owns the process-wide physical flash capability and exposes
//! hardware-level adapters used by higher-level backends:
//!
//! - serialized access to the single `esp_storage::FlashStorage` instance;
//! - the ESP NVS platform adapter over that shared flash;
//! - ESP partition-table lookup and raw partition erase helpers.
//!
//! It deliberately contains no ConfigSpace framing, OTA transaction state,
//! EWBT policy, deployment identity, or application persistence semantics.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_storage::nor_flash::{ErrorType, MultiwriteNorFlash, NorFlash, ReadNorFlash};
use esp_hal::peripherals::FLASH;
use esp_storage::FlashStorage;
use static_cell::StaticCell;

pub mod nvs;
pub mod partitions;

/// The one process-wide flash capability shared by ESP platform backends.
pub type SharedFlash = Mutex<CriticalSectionRawMutex, EspFlash>;

/// Exclusive access to the physical ESP flash.
pub struct EspFlash {
    storage: FlashStorage<'static>,
}

impl EspFlash {
    /// Access the underlying ESP storage driver while the shared capability is
    /// held exclusively by the caller.
    pub fn storage(&mut self) -> &mut FlashStorage<'static> {
        &mut self.storage
    }
}

static FLASH: StaticCell<SharedFlash> = StaticCell::new();

/// Construct the process-wide flash owner. Call exactly once per firmware image.
pub fn init(flash: FLASH<'static>) -> &'static SharedFlash {
    FLASH.init(Mutex::new(EspFlash {
        storage: FlashStorage::new(flash),
    }))
}

impl ErrorType for EspFlash {
    type Error = <FlashStorage<'static> as ErrorType>::Error;
}

impl ReadNorFlash for EspFlash {
    const READ_SIZE: usize = <FlashStorage<'static> as ReadNorFlash>::READ_SIZE;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.storage.read(offset, bytes)
    }

    fn capacity(&self) -> usize {
        self.storage.capacity()
    }
}

impl NorFlash for EspFlash {
    const WRITE_SIZE: usize = <FlashStorage<'static> as NorFlash>::WRITE_SIZE;
    const ERASE_SIZE: usize = <FlashStorage<'static> as NorFlash>::ERASE_SIZE;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.storage.erase(from, to)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.storage.write(offset, bytes)
    }
}

impl MultiwriteNorFlash for EspFlash {}
