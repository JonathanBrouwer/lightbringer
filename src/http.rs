use crate::web_app::AppRouter;
use crate::wifi::WifiStack;
use embassy_executor::Spawner;
use embassy_time::Duration;
use picoserve::*;
use static_cell::make_static;

const PORT: u16 = 80;
const MAX_CONNECTIONS: usize = 8;
pub(crate) const MAX_LISTENERS: usize = MAX_CONNECTIONS + 4;

pub async fn setup_http_server(
    stack: WifiStack,
    spawner: Spawner,
    app: &'static Router<AppRouter>,
) {
    let config = make_static!(Config::new(Timeouts {
        start_read_request: None,
        read_request: None,
        write: None,
    }));

    for id in 0..MAX_CONNECTIONS {
        spawner.must_spawn(web_task(id, stack, app, config));
    }
}

#[embassy_executor::task(pool_size = MAX_CONNECTIONS)]
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
        app,
        config,
        stack,
        PORT,
        &mut tcp_rx_buffer,
        &mut tcp_tx_buffer,
        &mut http_buffer,
    )
    .await
}
