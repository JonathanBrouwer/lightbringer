use crate::ota::read_ota;
use crate::partitions::find_partition_type;
use esp_partition_table::{AppPartitionType, DataPartitionType, PartitionEntry, PartitionType};

/// Find ota data partition
pub fn ota_data_part() -> PartitionEntry {
    find_partition_type(PartitionType::Data(DataPartitionType::Ota)).unwrap() //TODO
}

/// Find ota partition with certain sequence number
pub fn ota_part(seq: u8) -> PartitionEntry {
    let ota_part = find_partition_type(PartitionType::App(AppPartitionType::Ota(seq))).unwrap(); //TODO

    ota_part
}

/// Find ota sequence that was booted
pub fn booted_ota_seq() -> u32 {
    let data = read_ota();
    let seq = data.seq;
    assert!(seq < 16); // TODO
    seq
}
