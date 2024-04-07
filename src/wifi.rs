use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::clock::Clocks;
use esp_hal::peripherals::{RNG, SYSTIMER, WIFI};
use esp_hal::system::RadioClockControl;
use esp_hal::systimer::SystemTimer;
use esp_hal::Rng;
use esp_println::println;
use esp_wifi::wifi::{
    ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
    WifiState,
};
use esp_wifi::{initialize, EspWifiInitFor};
use static_cell::make_static;

const SSID: &str = "Jonathan's Tennisnet";
const PASSWORD: &str = "nahtanoj";
const MAX_SOCKETS: usize = 16;

pub type WifiStack = &'static Stack<WifiDevice<'static, WifiStaDevice>>;

pub async fn setup_wifi(
    systimer: SYSTIMER,
    rng: RNG,
    radio: RadioClockControl,
    clocks: &Clocks<'_>,
    wifi: WIFI,
    spawner: Spawner,
) -> WifiStack {
    let timer = SystemTimer::new(systimer).alarm0;
    let mut rng = Rng::new(rng);

    let init = initialize(EspWifiInitFor::Wifi, timer, rng, radio, &clocks).unwrap();
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    let config = Config::dhcpv4(Default::default());
    // Init network stack
    let stack = &*make_static!(Stack::new(
        wifi_interface,
        config,
        make_static!(StackResources::<MAX_SOCKETS>::new()),
        rng.random() as u64
    ));
    spawner.spawn(net_task(&stack)).ok();
    spawner.spawn(connect_task(controller)).ok();

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    stack
}

/// Task with the goal to connect to the wifi network when possible
#[embassy_executor::task]
async fn connect_task(mut controller: WifiController<'static>) {
    println!("Start connection task...");
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                println!("Disconnected from wifi, waiting 5 seconds before reconnecting...");
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi controller...");
            controller.start().await.unwrap();
        }
        println!("Trying to connect to wifi network...");

        match controller.connect().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

/// Task that runs the network stack
#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}
