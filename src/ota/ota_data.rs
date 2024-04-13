use crate::ota::ota_data_part;
use crate::ota::ota_data_structs::EspOTAData;
use embedded_storage::{ReadStorage, Storage};
use esp_storage::FlashStorage;

/// Read from ota data partition
pub fn read_ota() -> EspOTAData {
    let ota_data = ota_data_part();
    let mut flash = FlashStorage::new();
    let mut buffer = [0; 32];

    // Try first copy
    flash.read(ota_data.offset, &mut buffer).unwrap(); // TODO
    if let Ok(data) = EspOTAData::try_from(buffer) {
        return data;
    }

    // First copy is corrupted, try second one
    flash.read(ota_data.offset + 0x1000, &mut buffer).unwrap(); // TODO
    if let Ok(data) = EspOTAData::try_from(buffer) {
        return data;
    }

    unreachable!("OTA data corrupted") // TODO
}

/// Write to ota data partition
pub fn write_ota(data: EspOTAData) {
    let ota_data = ota_data_part();
    let mut flash = FlashStorage::new();
    let buffer: [u8; 32] = data.into();

    // Write first copy
    flash.write(ota_data.offset, &buffer).unwrap(); //TODO

    // Write second copy
    flash.write(ota_data.offset + 0x1000, &buffer).unwrap(); //TODO
}
