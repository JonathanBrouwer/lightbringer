use crate::partitions::ReadWritePartitionError::{PartitionFoundTwice, PartitionNotFound};
use esp_partition_table::{
    DataPartitionType, PartitionEntry, PartitionTable, PartitionType, StorageOpError,
};
use esp_println::println;
use esp_storage::FlashStorage;

#[derive(Debug)]
pub enum ReadWritePartitionError {
    StorageOpError(StorageOpError<FlashStorage>),
    PartitionNotFound,
    PartitionFoundTwice,
}

impl From<StorageOpError<FlashStorage>> for ReadWritePartitionError {
    fn from(value: StorageOpError<FlashStorage>) -> Self {
        Self::StorageOpError(value)
    }
}

const CALC_MD5: bool = false;

/// Find partition entry by name
pub fn find_partition_name(name: &str) -> Result<PartitionEntry, ReadWritePartitionError> {
    let table = PartitionTable::default();
    let mut flash = FlashStorage::new();
    let mut found_partition = None;

    for entry in table.iter_storage(&mut flash, CALC_MD5) {
        let ok_entry = entry?;
        if ok_entry.name() == name {
            if found_partition.is_none() {
                found_partition = Some(ok_entry);
            } else {
                return Err(PartitionFoundTwice);
            }
        }
    }

    found_partition.ok_or(PartitionNotFound)
}

/// Find partition entry by type
pub fn find_partition_type(typ: PartitionType) -> Result<PartitionEntry, ReadWritePartitionError> {
    let table = PartitionTable::default();
    let mut flash = FlashStorage::new();
    let mut found_partition = None;

    for entry in table.iter_storage(&mut flash, CALC_MD5) {
        let ok_entry = entry?;
        if ok_entry.type_ == typ {
            if found_partition.is_none() {
                found_partition = Some(ok_entry);
            } else {
                return Err(PartitionFoundTwice);
            }
        }
    }

    found_partition.ok_or(PartitionNotFound)
}
