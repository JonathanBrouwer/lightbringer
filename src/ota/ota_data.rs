use crate::ota::errors::OtaInternalError;
use crate::ota::errors::OtaInternalError::OtaDataCorrupt;
use crate::ota::ota_data_structs::EspOTAData;
use crate::ota::{ota_data_part, SECTOR_SIZE};
use embedded_storage::{ReadStorage, Storage};
use esp_storage::FlashStorage;

/// Read from ota data partition
pub fn read_ota_data() -> Result<EspOTAData, OtaInternalError> {
    match read_ota_data_both()? {
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
pub fn write_ota_data(data: EspOTAData) -> Result<(), OtaInternalError> {
    let ota_data = ota_data_part()?;
    let mut flash = FlashStorage::new();
    let buffer: [u8; 32] = data.into();

    let sector = match read_ota_data_both()? {
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

    flash.write(ota_data.offset + sector * SECTOR_SIZE as u32, &buffer)?;
    Ok(())
}

/// Read both ota partitions, return None if corrupt
fn read_ota_data_both() -> Result<(Option<EspOTAData>, Option<EspOTAData>), OtaInternalError> {
    let ota_data_part = ota_data_part()?;
    let mut flash = FlashStorage::new();
    let mut buffer = [0; 32];

    // Read first copy
    flash.read(ota_data_part.offset, &mut buffer)?;
    let ota_data0 = EspOTAData::try_from(buffer).ok();

    // Read second copy
    flash.read(ota_data_part.offset + SECTOR_SIZE as u32, &mut buffer)?;
    let ota_data1 = EspOTAData::try_from(buffer).ok();

    return Ok((ota_data0, ota_data1));
}
