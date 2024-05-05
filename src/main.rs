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

use core::panic::PanicInfo;
use build_time::build_time_local;
use crate::color_storage::{read_light_state, setup_color_storage};
use crate::wifi::setup_wifi;
use embassy_executor::Spawner;
use esp_hal::clock::Clocks;
use esp_hal::timer::TimerGroup;
use esp_hal::{
    clock::ClockControl,
    embassy::{self},
    peripherals::Peripherals,
    prelude::*,
    gpio::IO,
};
use esp_ota_nostd::ota_accept;
use esp_storage::FlashStorage;
use picoserve::Router;
use static_cell::make_static;

use crate::http::setup_http_server;
use crate::leds::setup_leds;
use crate::rotating_logger::RingBufferLogger;
use crate::value_synchronizer::ValueSynchronizer;
use crate::web_app::{make_app, AppRouter};

#[main]
async fn main(spawner: Spawner) {
    // Logging init
    let logger = RingBufferLogger::init();

    // Hardware init
    log::info!("Starting initialization of version {}...", build_time_local!("%Y-%m-%dT%H:%M:%S%.f%:z"));
    let peripherals = Peripherals::take();
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    // Setup GPIO pins
    let mut setup_pin = io.pins.gpio12.into_push_pull_output();
    setup_pin.set_high();
    io.pins.gpio13.into_push_pull_output().set_low();
    let mut red = io.pins.gpio18.into_push_pull_output();
    let mut blue = io.pins.gpio19.into_push_pull_output();
    red.set_low();
    blue.set_low();

    // Setup embassy
    let system = peripherals.SYSTEM.split();
    let clocks: &'static Clocks = make_static!(ClockControl::max(system.clock_control).freeze());
    let timer_group0 = TimerGroup::new_async(peripherals.TIMG0, clocks);
    embassy::init(clocks, timer_group0);

    // Setup app
    let initial_color = read_light_state();
    let value: &'static _ = make_static!(ValueSynchronizer::new(initial_color));
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
    let app: &'static Router<AppRouter> = make_static!(make_app(value, logger));
    let stack = setup_wifi(
        peripherals.SYSTIMER,
        peripherals.RNG,
        system.radio_clock_control,
        clocks,
        peripherals.WIFI,
        spawner,
    )
    .await;
    setup_http_server(stack, spawner, app).await;

    // Accept ota
    ota_accept(&mut FlashStorage::new()).unwrap();
    setup_pin.set_low();

    log::info!("Running...")
}

#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    let peripherals = unsafe { Peripherals::steal() };
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    io.pins.gpio13.into_push_pull_output().set_high();

    loop {}
}