use embassy_net::{Runner, Stack};
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::peripherals::{RADIO_CLK, RNG, SYSTIMER, WIFI};
use esp_hal::rng::Rng;
use esp_hal::timer::systimer::SystemTimer;
use esp_wifi::{init, EspWifiController};
use esp_wifi::wifi::{
    ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
    WifiState,
};
use crate::make_static;

const SSID: &str = "Jonathan's Tennisnet";
const PASSWORD: &str = "nahtanoj";
const MAX_SOCKETS: usize = 16;

pub async fn setup_wifi(
    systimer: SYSTIMER,
    rng: RNG,
    radio: RADIO_CLK,
    wifi: WIFI,
    spawner: Spawner,
) -> Stack<'static> {
    let timer = SystemTimer::new(systimer).alarm0;
    let mut rng = Rng::new(rng);
    let init: &'static EspWifiController<'static> = make_static!(EspWifiController<'static>, init(timer, rng, radio).unwrap());

    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    let config = Config::dhcpv4(Default::default());
    // Init network stack
    let (stack, runner): (Stack<'static>, Runner<_>) = embassy_net::new(
        wifi_interface,
        config,
        make_static!(StackResources<MAX_SOCKETS>, StackResources::<MAX_SOCKETS>::new()),
        (rng.random() as u64) << 32 | rng.random() as u64
    );
    spawner.spawn(connect_task(controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    log::info!("Waiting for network stack...");
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    log::info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            log::info!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    stack
}

/// Task with the goal to connect to the wifi network when possible
#[embassy_executor::task]
async fn connect_task(mut controller: WifiController<'static>) {
    log::info!("Start connection task...");

    loop {
        if let WifiState::StaConnected = esp_wifi::wifi::wifi_state() {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            log::info!("Disconnected from wifi, waiting 5 seconds before reconnecting...");
            Timer::after(Duration::from_millis(5000)).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();

            // On first setup, wait 2 seconds in order to avoid bootlooping bug
            Timer::after_secs(2).await;
            log::info!("Starting wifi controller...");
            controller.start().unwrap();
        }
        log::info!("Trying to connect to wifi network...");

        match controller.connect_async().await {
            Ok(_) => log::info!("Wifi connected!"),
            Err(e) => {
                log::info!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

/// Task that runs the network stack
#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) {
    runner.run().await
}
