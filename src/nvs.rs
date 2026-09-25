//! ESP NVS platform adapter over the shared physical flash.
//!
//! This module only bridges `esp-nvs` to `EspFlash`. Namespaces, keys,
//! record framing, quotas and health policy belong to consumers.

use embedded_storage::nor_flash::{ErrorType, MultiwriteNorFlash, NorFlash, ReadNorFlash};
use esp_nvs::platform::Crc;
use esp_nvs::Nvs;

use crate::EspFlash;

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

/// Borrowed NVS view of the shared ESP flash.
pub struct NvsFlash<'a>(&'a mut EspFlash);

impl NvsFlash<'_> {
    fn flash(&mut self) -> &mut EspFlash {
        self.0
    }
}

impl ErrorType for NvsFlash<'_> {
    type Error = <EspFlash as ErrorType>::Error;
}

impl ReadNorFlash for NvsFlash<'_> {
    const READ_SIZE: usize = <EspFlash as ReadNorFlash>::READ_SIZE;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.flash().read(offset, bytes)
    }

    fn capacity(&self) -> usize {
        self.0.capacity()
    }
}

impl NorFlash for NvsFlash<'_> {
    const WRITE_SIZE: usize = <EspFlash as NorFlash>::WRITE_SIZE;
    const ERASE_SIZE: usize = <EspFlash as NorFlash>::ERASE_SIZE;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.flash().erase(from, to)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.flash().write(offset, bytes)
    }
}

impl MultiwriteNorFlash for NvsFlash<'_> {}

impl Crc for NvsFlash<'_> {
    fn crc32(init: u32, data: &[u8]) -> u32 {
        <esp_storage::FlashStorage<'static> as Crc>::crc32(init, data)
    }
}

/// Construct an ESP NVS view over an already exclusively borrowed flash.
pub fn open(
    flash: &mut EspFlash,
    partition: NvsPartition,
) -> Result<Nvs<NvsFlash<'_>>, esp_nvs::error::Error> {
    Nvs::new(partition.offset, partition.size, NvsFlash(flash))
}
