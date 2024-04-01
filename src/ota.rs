use core::result::Result;
use crc::{Algorithm, Crc};
use embedded_io_async::{ErrorType, Read};
use esp_partition_table::{PartitionType, DataPartitionType, PartitionEntry, AppPartitionType};
use esp_storage::FlashStorage;
use crate::partitions::find_partition_type;
use embedded_storage::{ReadStorage, Storage};
use crate::ota::EspOTAState::{EspOtaImgAborted, EspOtaImgInvalid, EspOtaImgNew, EspOtaImgPendingVerify, EspOtaImgUndefined, EspOtaImgValid};

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
    ota_valid();
    let mut data_buffer = [0; 0x1000];
    let booted_seq = find_booted_ota_seq();
    let new_seq = (booted_seq + 1) % 2; // TODO: support more than 2 ota parts

    let read_len = new_data.read(&mut data_buffer).await.unwrap(); // TODO: propagate Read errors
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

/// Returns true if this OTA update has been accepted, i.e. with `ota_accept`
pub fn ota_valid() -> bool {
    let ota_data = find_ota_data();
    let mut flash = FlashStorage::new();
    let mut buffer = [0; 32];
    flash.read(ota_data.offset, &mut buffer).unwrap(); // TODO
    let data = EspOTAData::try_from(buffer).unwrap(); // TODO
    return match data.state {
        EspOtaImgValid => true,
        EspOtaImgUndefined => true,
        _ => false
    }
}

fn read_ota() -> EspOTAData {
    let ota_data = find_ota_data();
    let mut flash = FlashStorage::new();
    let mut buffer = [0; 32];

    // Try first copy
    flash.read(ota_data.offset, &mut buffer).unwrap(); // TODO
    if let Ok(data) = EspOTAData::try_from(buffer) {
        return data
    }

    // First copy is corrupted, try second one
    flash.read(ota_data.offset+0x1000, &mut buffer).unwrap(); // TODO
    if let Ok(data) = EspOTAData::try_from(buffer) {
        return data
    }

    unreachable!("OTA data corrupted") // TODO
}

fn write_ota(data: EspOTAData) {
    let ota_data = find_ota_data();
    let mut flash = FlashStorage::new();
    let mut buffer:[u8;32] = data.into();

    // Write first copy
    flash.write(ota_data.offset, &buffer).unwrap(); //TODO

    // Write second copy
    flash.write(ota_data.offset+0x1000, &buffer).unwrap(); //TODO
}

/// Copied from esp-idf
/// -`EspOtaImgNew`: Monitor the first boot. In bootloader this state is changed to EspOtaImgPendingVerify.
/// -`EspOtaImgPendingVerify`: First boot for this app was. If while the second boot this state is then it will be changed to EspOtaImgAborted.
/// -`EspOtaImgValid`: App was confirmed as workable. App can boot and work without limits.
/// -`EspOtaImgInvalid`: App was confirmed as non-workable. This app will not be selected to boot at all.
/// -`EspOtaImgAborted`: App could not confirm the workable or non-workable. In bootloader IMG_PENDING_VERIFY state will be changed to IMG_ABORTED. This app will not be selected to boot at all.
/// -`EspOtaImgUndefined`: Undefined. App can boot and work without limits.
#[derive(Debug,Copy,Clone,Eq,PartialEq)]
enum EspOTAState {
    EspOtaImgNew,
    EspOtaImgPendingVerify,
    EspOtaImgValid,
    EspOtaImgInvalid,
    EspOtaImgAborted,
    EspOtaImgUndefined,
}

/// Weak form of conversion, will return an error if unknown
impl TryFrom<u32> for EspOTAState {
    type Error = ();
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(EspOtaImgNew),
            1 => Ok(EspOtaImgPendingVerify),
            2 => Ok(EspOtaImgValid),
            3 => Ok(EspOtaImgInvalid),
            4 => Ok(EspOtaImgAborted),
            u32::MAX => Ok(EspOtaImgUndefined),
            _ => Err(())
        }
    }
}

impl From<EspOTAState> for u32 {
    fn from(value: EspOTAState) -> Self {
        match value {
            EspOtaImgNew => 0,
            EspOtaImgPendingVerify => 1,
            EspOtaImgValid => 2,
            EspOtaImgInvalid => 3,
            EspOtaImgAborted => 4,
            EspOtaImgUndefined => u32::MAX,
        }
    }
}

#[derive(Debug, Clone)]
struct EspOTAData {
    seq: u8,
    label: [u8; 20],
    state: EspOTAState,
    crc: u32,
}

impl TryFrom<[u8;32]> for EspOTAData {
    type Error = ();
    fn try_from(value: [u8; 32]) -> Result<Self, Self::Error> {
        let mut seq32 = u32::from_le_bytes(value[0..4].try_into().unwrap()); //TODO
        let seq = seq32.try_into().unwrap(); //TODO
        let label = value[4..24].try_into().unwrap(); //TODO
        let state = EspOTAState::try_from(u32::from_le_bytes(value[24..28].try_into().unwrap())).unwrap(); //TODO
        let crc = u32::from_le_bytes(value[28..32].try_into().unwrap()); //TODO
        return if crc == esp_crc32(&mut seq32.to_le_bytes()) {
            Ok(Self {
                seq,
                label,
                state,
                crc
            })
        } else {
            Err(()) //TODO
        }
    }
}

impl From<EspOTAData> for [u8;32] {
    fn from(value: EspOTAData) -> Self {
        let mut ret = [0;32];
        ret[0..4].copy_from_slice(&(value.seq as u32).to_le_bytes());
        ret[4..24].copy_from_slice(&value.label);
        ret[24..28].copy_from_slice(&u32::to_le_bytes(value.state.into()));
        let crc = esp_crc32(&mut (value.seq as u32).to_le_bytes());
        ret[28..32].copy_from_slice(&crc.to_le_bytes());
        return ret;
    }
}

fn find_ota_data() -> PartitionEntry {
    find_partition_type(
        PartitionType::Data(
            DataPartitionType::Ota
        )
    ).unwrap() //TODO
}

/// Find partition we booted from
fn find_booted_ota() -> PartitionEntry {
    let seq = find_booted_ota_seq();
    let ota_part = find_partition_type(
        PartitionType::App(
            AppPartitionType::Ota(seq)
        )).unwrap(); //TODO

    return ota_part
}

fn find_booted_ota_seq() -> u8 {
    let ota_data = find_ota_data();
    let mut flash = FlashStorage::new();
    let mut buffer = [0; 32];

    flash.read(ota_data.offset, &mut buffer).unwrap(); // TODO
    let seq = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
    assert!(seq < 16); // TODO
    seq.try_into().unwrap() // TODO
}

/// ESP32 CRC32 implementation (`esp_rom_crc32_le`)
/// This has only been verified to be identical with one input-output pair so use with caution.
fn esp_crc32(bytes: &mut [u8]) -> u32 {

    /// TODO: can this be a one-liner?
    for b in bytes.iter_mut() {
        *b = !*b;
    }

    !Crc::<u32>::new(&CRC_32_ESP).checksum(bytes)
}

const CRC_32_ESP: Algorithm<u32> = Algorithm {
    width: 32,
    poly: 0x04c11db7,
    init: u32::MAX,
    refin: true,
    refout: true,
    xorout: 0,
    check: 0x0000,
    residue: 0x0000
};