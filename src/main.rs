#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

mod color_storage;
mod http;
mod leds;
mod light_state;
mod ota;
mod partitions;
mod value_synchronizer;
mod web_app;
mod wifi;

use crate::color_storage::{read_light_state, setup_color_storage};
use crate::wifi::setup_wifi;
use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::clock::Clocks;
use esp_hal::timer::TimerGroup;
use esp_hal::{
    clock::ClockControl,
    embassy::{self},
    peripherals::Peripherals,
    prelude::*,
    IO,
};
use esp_println::println;
use esp_storage::FlashStorage;
use light_state::LightState;
use picoserve::Router;
use static_cell::make_static;

use crate::http::setup_http_server;
use crate::leds::setup_leds;
use crate::ota::{ota_accept, read_ota_data, write_ota_data};
use crate::value_synchronizer::ValueSynchronizer;
use crate::web_app::{make_app, AppRouter};

#[main]
async fn main(spawner: Spawner) {
    // Hardware init
    println!("Starting initialization...");
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks: &'static Clocks = make_static!(ClockControl::max(system.clock_control).freeze());
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, clocks);
    embassy::init(clocks, timer_group0);

    // Setup app
    let initial_color = read_light_state();
    let value: &'static _ = make_static!(ValueSynchronizer::new(initial_color));
    setup_color_storage(spawner, value);
    let app: &'static Router<AppRouter> = make_static!(make_app(value));

    // Setup leds
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    setup_leds(
        value,
        io.pins.gpio12,
        io.pins.gpio13,
        clocks,
        peripherals.LEDC,
        spawner,
    );

    // Setup http
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
    ota_accept();

    println!("Running...")
}
