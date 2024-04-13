use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Timer;
use embedded_storage::{ReadStorage, Storage};
use esp_println::println;
use esp_storage::FlashStorage;
use crate::value_synchronizer::ValueSynchronizer;
use crate::http::MAX_LISTENERS;
use crate::light_state::{LightState, LIGHT_STATE_LEN};
use crate::partitions::find_partition_name;

const WRITE_DELAY: u64 = 5;

pub fn read_light_state() -> LightState {
    let mut flash = FlashStorage::new();
    let partition = find_partition_name("userdata").unwrap();

    let mut buffer = [0; LIGHT_STATE_LEN];
    flash.read(partition.offset, &mut buffer).unwrap();

    // Uninitialized
    if buffer.iter().all(|v| *v == 255) {
        println!("Initializing to first-time light state.");
        return LightState::default()
    }

    LightState::from_bytes(&buffer)
}


pub fn setup_color_storage(spawner: Spawner, value: &'static ValueSynchronizer<MAX_LISTENERS, NoopRawMutex, LightState>) {
    spawner.must_spawn(storage_task(value));
}

#[embassy_executor::task]
async fn storage_task(
    value: &'static ValueSynchronizer<MAX_LISTENERS, NoopRawMutex, LightState>,
) -> ! {
    let mut flash = FlashStorage::new();
    let mut watcher = value.watch();
    let partition = find_partition_name("userdata").unwrap();
    loop {
        watcher.read().await;
        
        Timer::after_secs(WRITE_DELAY).await;

        watcher.skip().await;
        let message = value.read_clone();
        flash.write(partition.offset, &message.into_bytes()).unwrap();
        println!("Flash storage updated");
    }
}
