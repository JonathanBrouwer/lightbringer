#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

mod wifi;
mod http;
mod value_synchronizer;
mod web_app;

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::{clock::ClockControl, embassy::{self}, peripherals::Peripherals, prelude::*};
use esp_hal::timer::TimerGroup;
use esp_println::println;
use crate::wifi::setup_wifi;

use crate::http::setup_http_server;

#[main]
async fn main(spawner: Spawner) {
    // Hardware init
    println!("Starting initialization...");
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timer_group0);

    // Setup http
    let stack = setup_wifi(peripherals.SYSTIMER, peripherals.RNG, system.radio_clock_control, &clocks, peripherals.WIFI, spawner).await;
    setup_http_server(stack, spawner).await;

    println!("Running...")
}

