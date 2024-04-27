use esp_storage::FlashStorageError;
use crate::ota::errors::OtaInternalError::FlashError;
use crate::partitions::ReadWritePartitionError;

/// Errors that may occur during an OTA update
#[derive(Debug)]
pub enum OtaUpdateError<T> {
    /// Error while reading the update data
    ReadError(T),
    /// The image that was booted hasn't been verified as working yet,
    /// so it may not start an update before being verified.
    /// See `ota_accept`
    PendingVerify,
    /// Not enough space in partition
    OutOfSpace,
    /// Another update is already in progress
    AlreadyUpdating,
    /// Internal error
    InternalError(OtaInternalError),
}

#[derive(Debug)]
pub enum OtaInternalError {
    /// Corrupt ota data partition
    OtaDataCorrupt,
    /// Could not write to flash
    FlashError(FlashStorageError),
    /// Could not find partition
    PartitionError(ReadWritePartitionError)
}

impl From<ReadWritePartitionError> for OtaInternalError {
    fn from(value: ReadWritePartitionError) -> Self {
        Self::PartitionError(value)
    }
}

impl From<FlashStorageError> for OtaInternalError {
    fn from(value: FlashStorageError) -> Self {
        Self::FlashError(value)
    }
}

impl<T> From<FlashStorageError> for OtaUpdateError<T> {
    fn from(value: FlashStorageError) -> Self {
        Self::InternalError(FlashError(value))
    }
}

impl<T> From<OtaInternalError> for OtaUpdateError<T> {
    fn from(value: OtaInternalError) -> Self {
        Self::InternalError(value)
    }
}