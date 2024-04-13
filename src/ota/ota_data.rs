use embedded_storage::{ReadStorage, Storage};
use esp_storage::FlashStorage;
use crate::ota::errors::OtaInternalError;
use crate::ota::errors::OtaInternalError::OtaDataCorrupt;
use crate::ota::ota_data_structs::EspOTAData;
use crate::ota::ota_data_part;

/// Read from ota data partition
pub fn read_ota_data() -> Result<EspOTAData, OtaInternalError> {
    match read_ota_data_both() {
        (Some(data0), Some(data1)) => {
            if data0.seq > data1.seq {
                Ok(data0)
            } else {
                Ok(data1)
            }
        }
        // In case some data is corrupt
        (None, Some(data)) => Ok(data),
        (Some(data), None) => Ok(data),
        (None, None) => Err(OtaDataCorrupt),
    }
}

/// Write to ota data partition
pub fn write_ota_data(data: EspOTAData) {
    let ota_data = ota_data_part();
    let mut flash = FlashStorage::new();
    let buffer: [u8; 32] = data.into();

    let sector = match read_ota_data_both() {
        (Some(data0), Some(data1)) => {
            if data0.seq > data1.seq {
                1
            } else {
                0
            }
        }
        // In case some data is corrupt
        (None, Some(_data1)) => 0,
        (Some(_data0), None) => 1,
        (None, None) => 0,
    };

    flash.write(ota_data.offset + sector * 0x1000, &buffer).unwrap(); //TODO
}

/// Read both ota partitions, return None if corrupt
fn read_ota_data_both() -> (Option<EspOTAData>, Option<EspOTAData>) {
    let ota_data_part = ota_data_part();
    let mut flash = FlashStorage::new();
    let mut buffer = [0; 32];

    // Read first copy
    flash.read(ota_data_part.offset, &mut buffer).unwrap(); // TODO
    let ota_data0 = EspOTAData::try_from(buffer).ok(); // TODO

    // Read second copy
    flash.read(ota_data_part.offset + 0x1000, &mut buffer).unwrap(); // TODO
    let ota_data1 = EspOTAData::try_from(buffer).ok(); // TODO

    return (ota_data0, ota_data1)
}