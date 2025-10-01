#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

mod web_app;

use crate::web_app::{make_app, web_task, AppRouter};
use build_time::build_time_local;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::Level::{High, Low};
use esp_hal::gpio::{Output, OutputConfig};
use esp_hal::timer::timg::TimerGroup;
use esp_hal::Config;
use esp_hal_embassy::main;
use esp_ota_nostd::{get_booted_partition, ota_accept};
use esp_storage::FlashStorage;
use lightlib::color_storage::{read_light_state, setup_color_storage};
use lightlib::http::setup_http_server;
use lightlib::http::MAX_LISTENERS;
use lightlib::leds::setup_leds;
use lightlib::light_state::LightState;
use lightlib::rotating_logger::RingBufferLogger;
use lightlib::value_synchronizer::ValueSynchronizer;
use lightlib::wifi::setup_wifi;
use picoserve::{make_static, Router};
use lightlib::time::ntp_task;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
async fn main(spawner: Spawner) {
    // Logging init
    let logger = RingBufferLogger::init();

    // Hardware init
    let mut storage = FlashStorage::new();
    let partition = get_booted_partition(&mut storage).unwrap();
    log::info!(
        "Starting initialization from partition {} with build time {}...",
        partition.name(),
        build_time_local!("%Y-%m-%dT%H:%M:%S%.f%:z")
    );
    let peripherals = esp_hal::init(Config::default().with_cpu_clock(CpuClock::max()));
    esp_alloc::heap_allocator!(size: 128 * 1024);

    // Setup GPIO pins
    let output_config = OutputConfig::default();
    let mut setup_pin = Output::new(peripherals.GPIO12, High, output_config);
    let _debug_pin = Output::new(peripherals.GPIO13, Low, output_config);
    let red = peripherals.GPIO0;
    let blue = peripherals.GPIO1;

    // Setup embassy
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    // Setup app
    let initial_color = read_light_state();
    let value = make_static!(ValueSynchronizer<MAX_LISTENERS, NoopRawMutex, LightState>, ValueSynchronizer::new(initial_color));
    setup_color_storage(spawner, value);

    // Setup leds
    setup_leds(value, red, blue, peripherals.LEDC, spawner);

    // Setup http
    let app = make_static!(Router<AppRouter>, make_app(value, logger));
    let stack = setup_wifi(
        peripherals.SYSTIMER,
        peripherals.RNG,
        peripherals.WIFI,
        spawner,
    )
    .await;

    setup_http_server(stack, spawner, app, web_task).await;

    // Accept ota
    ota_accept(&mut storage).unwrap();
    setup_pin.set_low();

    // Get ntp
    if spawner.spawn(ntp_task(stack)).is_err() { log::warn!("ntp_task failed to spawn"); };

    log::info!("Running...")
}
