//! ESP partition-table and raw partition helpers.
//!
//! The functions here are intentionally policy-free. They locate a requested
//! ESP-IDF partition and perform bounded raw erase operations. Slot selection,
//! rollback and transaction semantics remain in higher-level consumers.

use embedded_storage::Storage;
use embedded_storage::nor_flash::NorFlash;
use esp_bootloader_esp_idf::partitions::{
    PARTITION_TABLE_MAX_LEN, PartitionType, read_partition_table,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartitionRange {
    pub offset: u32,
    pub size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionError {
    TableUnreadable,
    NotFound,
    AddressOverflow,
    Unaligned,
    Flash,
}

pub const TABLE_BUFFER_SIZE: usize = PARTITION_TABLE_MAX_LEN;

/// Locate one partition of the requested ESP-IDF type.
pub fn find<F>(
    flash: &mut F,
    table_buffer: &mut [u8; PARTITION_TABLE_MAX_LEN],
    kind: PartitionType,
) -> Result<PartitionRange, PartitionError>
where
    F: Storage,
{
    let table = read_partition_table(flash, table_buffer)
        .map_err(|_| PartitionError::TableUnreadable)?;
    let entry = table
        .find_partition(kind)
        .map_err(|_| PartitionError::TableUnreadable)?
        .ok_or(PartitionError::NotFound)?;

    Ok(PartitionRange {
        offset: entry.offset(),
        size: entry.len() as usize,
    })
}

/// Erase a logical, erase-aligned range inside a previously located partition.
pub fn erase_range<F>(
    flash: &mut F,
    partition: PartitionRange,
    logical_from: u64,
    logical_to: u64,
) -> Result<(), PartitionError>
where
    F: NorFlash,
{
    if logical_to < logical_from {
        return Err(PartitionError::AddressOverflow);
    }

    let from = usize::try_from(logical_from).map_err(|_| PartitionError::AddressOverflow)?;
    let to = usize::try_from(logical_to).map_err(|_| PartitionError::AddressOverflow)?;

    if from % F::ERASE_SIZE != 0 || to % F::ERASE_SIZE != 0 {
        return Err(PartitionError::Unaligned);
    }
    if to > partition.size {
        return Err(PartitionError::AddressOverflow);
    }
    if from == to {
        return Ok(());
    }

    let base = usize::try_from(partition.offset).map_err(|_| PartitionError::AddressOverflow)?;
    let absolute_from = base.checked_add(from).ok_or(PartitionError::AddressOverflow)?;
    let absolute_to = base.checked_add(to).ok_or(PartitionError::AddressOverflow)?;

    flash
        .erase(
            u32::try_from(absolute_from).map_err(|_| PartitionError::AddressOverflow)?,
            u32::try_from(absolute_to).map_err(|_| PartitionError::AddressOverflow)?,
        )
        .map_err(|_| PartitionError::Flash)
}
