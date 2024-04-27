mod crc;
mod errors;
mod ota_data;
mod ota_data_structs;
mod partition;

pub use crate::ota::errors::OtaError;
pub use crate::ota::ota_data::{read_ota_data, write_ota_data};
use crate::ota::ota_data_structs::{EspOTAData, EspOTAState};
use crate::ota::partition::{ota_data_part, ota_part};
use core::result::Result;
use core::sync::atomic::{AtomicBool, Ordering};
use embedded_io_async::Read;
use embedded_storage::Storage;
use esp_storage::FlashStorage;

/// Size of a flash sector
const SECTOR_SIZE: usize = 0x1000;

static IS_UPDATING: AtomicBool = AtomicBool::new(false);

/// Begin a new OTA update.
/// N.B. a new update can only be started after the currently running firmware has been verified!
/// See `ota_accept`.
/// Pass a stream of u8 to serve as the new binary.
/// May return an `OtaError`, or return successfully
/// If the update was successful, the caller should reboot to activate the new firmware
pub async fn ota_begin<R: Read>(mut new_data: R) -> Result<(), OtaError<R::Error>> {
    // Safety: IS_UPDATING is not accessible to interrupts and the ESP32C3 chip is single-core
    // Safe since there is no await point between loading and storing
    // TODO check this if we add other chip support
    if IS_UPDATING.load(Ordering::SeqCst) {
        return Err(OtaError::AlreadyUpdating);
    }
    IS_UPDATING.store(true, Ordering::SeqCst);

    if !ota_valid() {
        return Err(OtaError::PendingVerify);
    }
    let ota_data = read_ota_data().unwrap(); //TODO
    let booted_seq = ota_data.seq - 1;
    let new_seq = ota_data.seq + 1; // TODO: support more than 2 ota partitions
    log::info!("Currently running from {booted_seq}, writing to {new_seq}");
    let ota_app = ota_part(((new_seq - 1) % 2) as u8);

    let mut data_written = 0;
    let mut flash = FlashStorage::new();
    // Write new ota to flash
    loop {
        let mut data_buffer = [0; SECTOR_SIZE];
        let mut read_len = 0;

        let mut is_done = false;
        while read_len < SECTOR_SIZE {
            let read = new_data.read(&mut data_buffer[read_len..]).await.unwrap();
            if read == 0 {
                is_done = true;
                break;
            }
            read_len += read;
        }

        if data_written + read_len > ota_app.size {
            return Err(OtaError::OutOfSpace);
        }
        log::info!("Wrote {data_written:x} so far...");
        flash
            .write(
                ota_app.offset + data_written as u32,
                &data_buffer[0..read_len],
            )
            .unwrap(); // TODO
        data_written += read_len;

        if is_done {
            break;
        }
    }

    // Write new OTA data boot entry
    let data = EspOTAData::new(new_seq, [0xFF; 20]);
    write_ota_data(data);

    Ok(())
}

/// Mark OTA update as valid.
/// Must be called after an OTA update to confirm the new firmware works.
/// May also be called after a reboot without OTA.
/// If the system reboots before an OTA update is confirmed
/// the update will be marked as aborted and will not be booted again.
pub fn ota_accept() {
    let mut data = read_ota_data().unwrap(); //TODO
    data.state = EspOTAState::Valid;
    write_ota_data(data);
}

/// Explicitly mark an OTA update as invalid.
/// May be called after an OTA update, but is not required.
/// If the system reboots before an OTA update is confirmed as valid
/// the update will be marked as aborted and will not be booted again.
pub fn ota_reject() {
    let mut data = read_ota_data().unwrap(); //TODO
    data.state = EspOTAState::Invalid;
    write_ota_data(data);
}

/// Returns true if this OTA update has been accepted, i.e. with `ota_accept`
pub fn ota_valid() -> bool {
    let data = read_ota_data().unwrap(); //TODO
    match data.state {
        EspOTAState::Valid => true,
        EspOTAState::Undefined => true,
        _ => false,
    }
}
