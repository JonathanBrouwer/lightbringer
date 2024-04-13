mod crc;
mod errors;
mod ota_data;
mod ota_data_structs;
mod partition;

pub use crate::ota::errors::OtaError;
pub use crate::ota::ota_data::{read_ota, write_ota};
use crate::ota::ota_data_structs::{EspOTAData, EspOTAState};
use crate::ota::partition::{ota_data_part, ota_part};
use core::result::Result;
use embedded_io_async::Read;
use embedded_storage::Storage;
use esp_println::println;
use esp_storage::FlashStorage;

/// Begin a new OTA update.
/// N.B. a new update can only be started after the currently running firmware has been verified!
/// See `ota_accept`.
/// Pass a stream of u8 to serve as the new binary.
/// May return an `OtaError`, or return successfully
/// If the update was successful, the caller should reboot to activate the new firmware
pub async fn ota_begin<R: Read>(mut new_data: R) -> Result<(), OtaError<R::Error>> {
    if !ota_valid() {
        return Err(OtaError::PendingVerify);
    }
    let booted_seq = 1; //find_booted_ota_seq() - 1;
    let new_seq = booted_seq + 1; //(booted_seq + 1) % 2; // TODO: support more than 2 ota partitions
    let ota_app = ota_part(new_seq as u8 - 1);

    println!("Currently running from {booted_seq}, writing to {new_seq}");

    // Write new ota to flash
    let mut data_buffer = [0; 0x1000];
    let mut data_written = 0;
    let mut flash = FlashStorage::new();

    while let Ok(read_len) = new_data.read(&mut data_buffer).await {
        // TODO: propagate the read errors
        if read_len == 0 {
            break;
        }
        if data_written + read_len > ota_app.size {
            return Err(OtaError::OutOfSpace);
        }
        println!("Wrote {data_written} so far...");
        flash
            .write(
                ota_app.offset + data_written as u32,
                &data_buffer[0..read_len],
            )
            .unwrap(); // TODO
        data_written += read_len;
    }

    // Write new OTA data boot entry
    let data = EspOTAData::new(new_seq, [0xED; 20]);
    write_ota(data);

    Ok(())
}

/// Mark OTA update as valid.
/// Must be called after an OTA update to confirm the new firmware works.
/// May also be called after a reboot without OTA.
/// If the system reboots before an OTA update is confirmed
/// the update will be marked as aborted and will not be booted again.
pub fn ota_accept() {
    let mut data = read_ota();
    data.state = EspOTAState::Valid;
    write_ota(data);
}

/// Explicitly mark an OTA update as invalid.
/// May be called after an OTA update, but is not required.
/// If the system reboots before an OTA update is confirmed as valid
/// the update will be marked as aborted and will not be booted again.
pub fn ota_reject() {
    let mut data = read_ota();
    data.state = EspOTAState::Invalid;
    write_ota(data);
}

/// Returns true if this OTA update has been accepted, i.e. with `ota_accept`
pub fn ota_valid() -> bool {
    let data = read_ota();
    match data.state {
        EspOTAState::Valid => true,
        EspOTAState::Undefined => true,
        _ => false,
    }
}
