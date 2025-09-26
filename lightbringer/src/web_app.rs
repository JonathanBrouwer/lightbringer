use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Duration;
use lightlib::http::{MAX_CONNECTIONS, MAX_LISTENERS, PORT};
use lightlib::light_state::LightState;
use lightlib::rotating_logger::RingBufferLogger;
use lightlib::value_synchronizer::ValueSynchronizer;
use lightlib::web_app::common_routes;
use lightlib::web_app::ColorHandler;
use picoserve::response::WebSocketUpgrade;
use picoserve::routing::{get, get_service, PathRouter};
use picoserve::{listen_and_serve, response, Config, Router};

pub type AppRouter = impl PathRouter;

#[define_opaque(AppRouter)]
pub fn make_app(
    data: &'static ValueSynchronizer<MAX_LISTENERS, NoopRawMutex, LightState>,
    logger: &'static RingBufferLogger,
) -> Router<AppRouter> {
    common_routes(logger)
        .route(
            "/",
            get_service(response::File::html(include_str!(
                "../resources/index.html"
            ))),
        )
        .route(
            "/ws",
            get(move |update: WebSocketUpgrade| update.on_upgrade(ColorHandler { color: data })),
        )
}

#[embassy_executor::task(pool_size = MAX_CONNECTIONS)]
pub async fn web_task(
    id: usize,
    stack: Stack<'static>,
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
