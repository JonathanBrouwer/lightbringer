use core::result::Result;
use embedded_io_async::Read;

/// Errors that may occur during an OTA update
#[derive(Debug,Clone)]
pub enum OtaError<T> {
    /// The image that was booted hasn't been verified as working yet,
    /// so it may not start an update before being verified.
    /// See `ota_verify`
    PendingVerify,
    /// Error while reading the update data
    ReadError(T),
}

/// Begin a new OTA update.
/// Pass a stream of u8 to serve as the new binary.
/// May return an `OtaError`, or return successfully
/// If the update was successful, the caller should reboot to activate the new firmware
pub async fn ota_begin<R: Read>(mut new_data: R) -> Result<(), OtaError<R::Error>> {
    let mut data_buffer = [0; 0x1000];
    let read_len = new_data.read(&mut data_buffer).await?;
    Ok(())
}

/// Mark OTA update as valid.
/// Must be called after an OTA update to confirm the new firmware works.
/// May also be called after a reboot without OTA.
/// If the system reboots before an OTA update is confirmed
/// the update will be marked as aborted and will not be booted again.
pub fn ota_accept() {

}

/// Explicitly mark an OTA update as invalid.
/// May be called after an OTA update, but is not required.
/// If the system reboots before an OTA update is confirmed
/// the update will be marked as aborted and will not be booted again.
pub fn ota_reject() {

}