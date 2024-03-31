#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

mod wifi;
mod http;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{clock::ClockControl, embassy::{self}, peripherals::Peripherals, prelude::*};
use esp_hal::timer::TimerGroup;
use esp_println::println;
use crate::wifi::setup_wifi;

use embedded_storage::{ReadStorage, Storage};
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

    let stack = setup_wifi(peripherals.SYSTIMER, peripherals.RNG, system.radio_clock_control, &clocks, peripherals.WIFI, spawner).await;
    setup_http_server(stack, spawner).await;

    // let mut flash = FlashStorage::new();
    // let table = PartitionTable::new(0x8000, 0x1000);
    // table.read_storage(&mut flash, None);

    loop {
        println!("Ping!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

