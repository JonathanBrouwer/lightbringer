use core::fmt::{Display, Formatter, Write};
use core::ptr::read;
use core::result::Result;
use crc::{Algorithm, Crc};
use embedded_io_async::{ErrorType, Read};
use esp_partition_table::{PartitionType, DataPartitionType, PartitionEntry, AppPartitionType};
use esp_storage::FlashStorage;
use crate::partitions::find_partition_type;
use embedded_storage::{ReadStorage, Storage};
use esp_println::println;

/// Errors that may occur during an OTA update
#[derive(Debug,Clone)]
pub enum OtaError<T> {
    /// The image that was booted hasn't been verified as working yet,
    /// so it may not start an update before being verified.
    /// See `ota_accept`
    PendingVerify,
    /// Error while reading the update data
    ReadError(T),
    /// Not enough space in partition
    OutOfSpace,
}

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
    let booted_seq = find_booted_ota_seq();
    let new_seq = (booted_seq + 1) % 2; // TODO: support more than 2 ota partitions
    let ota_app = find_ota(new_seq);

    // Write new ota to flash
    let mut data_buffer = [0; 0x1000];
    let mut data_written = 0;
    let mut flash = FlashStorage::new();

    while let Some(read_len) = new_data.read(&mut data_buffer).await {
        if read_len == 0 {
            break;
        }
        if data_written + read_len > ota_app.size {
            return Err(OtaError::OutOfSpace)
        }
        flash.write(ota_app.offset+data_written, new_data[0..read_len]).unwrap(); // TODO: propagate Read errors
        data_written += read_len;
    }

    // Write new OTA data boot entry
    let data = EspOTAData::new(new_seq, [0xff;20]);
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
    return match data.state {
        EspOTAState::Valid => true,
        EspOTAState::Undefined => true,
        _ => false
    }
}

/// Read from ota data partition
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

/// Write to ota data partition
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
/// -`New`: Monitor the first boot. In bootloader this state is changed to PendingVerify.
/// -`PendingVerify`: First boot for this app was. If while the second boot this state is then it will be changed to Aborted.
/// -`Valid`: App was confirmed as workable. App can boot and work without limits.
/// -`Invalid`: App was confirmed as non-workable. This app will not be selected to boot at all.
/// -`Aborted`: App could not confirm the workable or non-workable. In bootloader IMG_PENDING_VERIFY state will be changed to IMG_ABORTED. This app will not be selected to boot at all.
/// -`Undefined`: Undefined. App can boot and work without limits.
#[derive(Debug,Copy,Clone,Eq,PartialEq)]
enum EspOTAState {
    New,
    PendingVerify,
    Valid,
    Invalid,
    Aborted,
    Undefined,
}

/// Weak form of conversion, will return an error if unknown
impl TryFrom<u32> for EspOTAState {
    type Error = ();
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::New),
            1 => Ok(Self::Verify),
            2 => Ok(Self::Valid),
            3 => Ok(Self::Invalid),
            4 => Ok(Self::Aborted),
            u32::MAX => Ok(Self::Undefined),
            _ => Err(())
        }
    }
}

impl From<EspOTAState> for u32 {
    fn from(value: EspOTAState) -> Self {
        match value {
            New => 0,
            PendingVerify => 1,
            Valid => 2,
            Invalid => 3,
            Aborted => 4,
            Undefined => u32::MAX,
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

impl EspOTAData {
    fn new(seq: u8, label: [u8; 20]) -> Self {
        let state = EspOTAState::PendingVerify;
        let crc = esp_crc32(&(seq as u32).to_le_bytes());
        Self {
            seq,
            label,
            state,
            crc,
        }
    }
}

impl Display for EspOTAData {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "EspOTAData {{ seq: {}, label: {:x?}, state: {:?}, crc: 0x{:08x} }}",
               self.seq,
               self.label,
               self.state,
               self.crc)
    }
}

impl TryFrom<[u8;32]> for EspOTAData {
    type Error = ();
    fn try_from(value: [u8; 32]) -> Result<Self, Self::Error> {
        let mut seq32 = u32::from_le_bytes(value[0..4].try_into().unwrap()); //TODO
        let seq = seq32.try_into().unwrap(); //TODO
        let label = value[4..24].try_into().unwrap(); //TODO
        let state = EspOTAState::try_from(u32::from_le_bytes(value[24..28].try_into().unwrap())).unwrap(); //TODO
        let crc = u32::from_le_bytes(value[28..32].try_into().unwrap()); //TODO
        return if crc == esp_crc32(&seq32.to_le_bytes()) {
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
        let crc = esp_crc32(&(value.seq as u32).to_le_bytes());
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

/// Find ota partition with certain sequence number
fn find_ota(seq: u8) -> PartitionEntry {
    let ota_part = find_partition_type(
        PartitionType::App(
            AppPartitionType::Ota(seq)
        )).unwrap(); //TODO

    return ota_part
}

/// Find ota sequence that was booted
fn find_booted_ota_seq() -> u8 {
    let data = read_ota();
    let seq = data.seq;
    assert!(seq < 16); // TODO
    seq
}

/// ESP32 CRC32 implementation (`esp_rom_crc32_le`)
/// This has only been verified to be identical with one input-output pair so use with caution.
fn esp_crc32(bytes: &[u8]) -> u32 {

    /// TODO: can this be a one-liner?
    let mut cloned_bytes = bytes.clone();
    for b in cloned_bytes.iter_mut() {
        *b = !*b;
    }

    !Crc::<u32>::new(&CRC_32_ESP).checksum(cloned_bytes)
}

const CRC_32_ESP: Algorithm<u32> = Algorithm {
    width: 32,
    poly: 0x04c11db7,
    init: u32::MAX,
    refin: true,
    refout: true,
    xorout: 0,
    check: 0,
    residue: 0
};