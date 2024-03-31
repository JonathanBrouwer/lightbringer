#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

mod wifi;
mod ota;
mod partitions;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{clock::ClockControl, embassy::{self}, peripherals::Peripherals, prelude::*};
use esp_hal::timer::TimerGroup;
use esp_println::println;
use crate::wifi::setup_wifi;


#[main]
async fn main(spawner: Spawner) {
    // Hardware init
    println!("Starting initialization...");
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timer_group0);

    setup_wifi(peripherals.SYSTIMER, peripherals.RNG, system.radio_clock_control, &clocks, peripherals.WIFI, &spawner).await;

    loop {
        println!("Bing!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

