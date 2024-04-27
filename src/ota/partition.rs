use crate::partitions::find_partition_type;
use esp_partition_table::{AppPartitionType, DataPartitionType, PartitionEntry, PartitionType};
use crate::ota::errors::OtaInternalError;

/// Find ota data partition
pub fn ota_data_part() -> Result<PartitionEntry, OtaInternalError> {
    Ok(find_partition_type(PartitionType::Data(DataPartitionType::Ota))?)
}

/// Find ota partition with certain sequence number
pub fn ota_part(seq: u8) -> Result<PartitionEntry, OtaInternalError> {
    Ok(find_partition_type(PartitionType::App(AppPartitionType::Ota(seq)))?)
}
