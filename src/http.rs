use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_time::Duration;
use esp_wifi::wifi::WifiStaDevice;
use picoserve::routing::{get, PathRouter};
use picoserve::*;
use static_cell::make_static;
use crate::wifi::WifiStack;

const PORT: u16 = 8;
const WEB_TASK_POOL_SIZE: usize = 8;

pub async fn setup_http_server(stack: WifiStack, spawner: Spawner) {
    let app = make_static!(make_app());
    let config = make_static!(Config::new(Timeouts {
        start_read_request: None,
        read_request: None,
        write: None,
    }));

    spawner.must_spawn(web_task(0, stack, app, config));
}

type AppRouter = impl PathRouter;
fn make_app() -> picoserve::Router<AppRouter> {
    picoserve::Router::new().route("/", get(|| async move { "Hello World" }))
}

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
async fn web_task(
    id: usize,
    stack: WifiStack,
    app: &'static Router<AppRouter>,
    config: &'static Config<Duration>,
) -> ! {
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    listen_and_serve(
        id,
        &app,
        config,
        stack,
        80,
        &mut tcp_rx_buffer,
        &mut tcp_tx_buffer,
        &mut http_buffer,
    ).await
}