use crate::http::MAX_LISTENERS;
use crate::light_state::LightState;
use crate::value_synchronizer::ValueSynchronizer;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Timer as EmbassyTimer;
use esp_hal::clock::Clocks;
use esp_hal::gpio::GpioPin;
use esp_hal::ledc::channel::config::PinConfig;
use esp_hal::ledc::channel::Channel;
use esp_hal::ledc::timer::config::Duty;
use esp_hal::ledc::timer::config::Duty::Duty12Bit;
use esp_hal::ledc::{channel, timer, LSGlobalClkSource, LowSpeed, Ledc};
use esp_hal::ledc::timer::Timer;
use esp_hal::prelude::{_esp_hal_ledc_channel_ChannelHW, _esp_hal_ledc_channel_ChannelIFace, _esp_hal_ledc_timer_TimerIFace, _fugit_RateExtU32};
use crate::make_static;

pub const PIN_RED: u8 = 18;
pub const PIN_BLUE: u8 = 1;
pub const DUTY: Duty = Duty12Bit;

const STARTUP_DELAY: u64 = 0;
const FADE_IN_TIME: u64 = 2000;
const STEPS: u64 = 100;

pub fn setup_leds(
    value: &'static ValueSynchronizer<MAX_LISTENERS, NoopRawMutex, LightState>,
    red: GpioPin<PIN_RED>,
    blue: GpioPin<PIN_BLUE>,
    clocks: &'static Clocks<'_>,
    ledc: esp_hal::peripherals::LEDC,
    spawner: Spawner,
) {
    let red = make_static!(GpioPin<PIN_RED>, red);
    let blue = make_static!(GpioPin<PIN_BLUE>, blue);

    let ledc = make_static!(Ledc, Ledc::new(ledc, clocks,));
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let timer: &'static mut _ = make_static!(Timer<LowSpeed>, ledc.get_timer::<LowSpeed>(timer::Number::Timer1));
    timer
        .configure(timer::config::Config {
            duty: DUTY,
            clock_source: timer::LSClockSource::APBClk,
            frequency: (80_000_000 >> (DUTY as u32)).Hz(),
        })
        .unwrap();

    let mut red_channel = ledc.get_channel(channel::Number::Channel0, red);
    red_channel
        .configure(channel::config::Config {
            timer,
            duty_pct: 0,
            pin_config: PinConfig::PushPull,
        })
        .unwrap();

    let mut blue_channel = ledc.get_channel(channel::Number::Channel1, blue);
    blue_channel
        .configure(channel::config::Config {
            timer,
            duty_pct: 0,
            pin_config: PinConfig::PushPull,
        })
        .unwrap();

    spawner.must_spawn(led_task(value, red_channel, blue_channel));
}

#[embassy_executor::task]
async fn led_task(
    value: &'static ValueSynchronizer<MAX_LISTENERS, NoopRawMutex, LightState>,
    red_channel: Channel<'static, LowSpeed, GpioPin<PIN_RED>>,
    blue_channel: Channel<'static, LowSpeed, GpioPin<PIN_BLUE>>,
) -> ! {
    let mut watcher = value.watch();

    // Initial update
    let message = value.read_clone();
    let red = (message.warm as u32) << (DUTY as u32) >> 16;
    let blue = (message.cold as u32) << (DUTY as u32) >> 16;

    // Wait with starting
    EmbassyTimer::after_millis(STARTUP_DELAY).await;
    log::info!("Fading in leds...");
    for i in 1..=STEPS {
        EmbassyTimer::after_millis(FADE_IN_TIME / STEPS).await;
        red_channel.set_duty_hw(red * i as u32 / STEPS as u32);
        blue_channel.set_duty_hw(blue * i as u32 / STEPS as u32);
    }

    log::info!("Initial color set to {red} {blue}");

    loop {
        let message = watcher.read().await;

        let red = (message.warm as u32) << (DUTY as u32) >> 16;
        let blue = (message.cold as u32) << (DUTY as u32) >> 16;

        red_channel.set_duty_hw(red);
        blue_channel.set_duty_hw(blue);

        log::info!("Color set to {red} {blue}");
    }
}
