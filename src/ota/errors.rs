/// Errors that may occur during an OTA update
#[derive(Debug, Clone)]
pub enum OtaError<T> {
    /// The image that was booted hasn't been verified as working yet,
    /// so it may not start an update before being verified.
    /// See `ota_accept`
    PendingVerify,
    /// Error while reading the update data
    ReadError(T),
    /// Not enough space in partition
    OutOfSpace,
    /// Internal error
    InternalError(OtaInternalError)
}

#[derive(Debug, Clone)]
pub enum OtaInternalError {
    /// Corrupt ota data partition
    OtaDataCorrupt,
}