#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

mod color_storage;
mod http;
mod leds;
mod light_state;
mod value_synchronizer;
mod web_app;
mod wifi;
mod rotating_logger;
mod make_static;

use esp_backtrace as _;
use build_time::build_time_local;
use crate::color_storage::{read_light_state, setup_color_storage};
use crate::wifi::setup_wifi;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use esp_hal::clock::Clocks;
use esp_hal::{
    clock::ClockControl,
    peripherals::Peripherals,
    prelude::*,
};
use esp_hal::gpio::{Io, Output};
use esp_hal::gpio::Level::{High, Low};
use esp_hal::system::SystemControl;
use esp_hal::timer::timg::TimerGroup;
use esp_ota_nostd::{get_booted_partition, ota_accept};
use esp_storage::FlashStorage;
use picoserve::Router;

use crate::http::setup_http_server;
use crate::leds::setup_leds;
use crate::rotating_logger::RingBufferLogger;
use crate::value_synchronizer::ValueSynchronizer;
use crate::web_app::{make_app, AppRouter};
use crate::http::MAX_LISTENERS;
use crate::light_state::LightState;

#[main]
async fn main(spawner: Spawner) {
    // Logging init
    let logger = RingBufferLogger::init();

    // Hardware init
    let mut storage = FlashStorage::new();
    let partition = get_booted_partition(&mut storage).unwrap();
    log::info!("Starting initialization from partition {} with build time {}...", partition.name(), build_time_local!("%Y-%m-%dT%H:%M:%S%.f%:z"));
    let peripherals = Peripherals::take();
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // Setup GPIO pins
    let mut setup_pin = Output::new(io.pins.gpio12, High);
    let _debug_pin = Output::new(io.pins.gpio13, Low);
    let red = io.pins.gpio0;
    let blue = io.pins.gpio1;

    // Setup embassy
    let system = SystemControl::new(peripherals.SYSTEM);
    let clock_control = ClockControl::max(system.clock_control).freeze();
    let clocks = make_static!(Clocks, clock_control);
    let timer_group0 = TimerGroup::new_async(peripherals.TIMG0, clocks);
    esp_hal_embassy::init(clocks, timer_group0);

    // Setup app
    let initial_color = read_light_state();
    let value = make_static!(ValueSynchronizer<MAX_LISTENERS, NoopRawMutex, LightState>, ValueSynchronizer::new(initial_color));
    setup_color_storage(spawner, value);

    // Setup leds
    setup_leds(
        value,
        red,
        blue,
        clocks,
        peripherals.LEDC,
        spawner,
    );

    // Setup http
    let app = make_static!(Router<AppRouter>, make_app(value, logger));
    let stack = setup_wifi(
        peripherals.SYSTIMER,
        peripherals.RNG,
        peripherals.RADIO_CLK,
        clocks,
        peripherals.WIFI,
        spawner,
    )
    .await;
    setup_http_server(stack, spawner, app).await;

    // Accept ota
    ota_accept(&mut storage).unwrap();
    setup_pin.set_low();

    log::info!("Running...")
}

