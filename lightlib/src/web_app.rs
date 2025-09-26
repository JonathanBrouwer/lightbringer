use crate::http::MAX_LISTENERS;
use crate::light_state::{LightState, LIGHT_STATE_LEN};
use crate::rotating_logger::RingBufferLogger;
use crate::value_synchronizer::ValueSynchronizer;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embedded_io_async::{Read, Write};
use esp_hal::system::software_reset;
use esp_ota_nostd::ota_begin;
use esp_storage::FlashStorage;
use picoserve::request::Request;
use picoserve::response::ws::{Message, SocketRx, SocketTx, WebSocketCallback};
use picoserve::response::{IntoResponse, ResponseWriter, WebSocketUpgrade};
use picoserve::routing::{get, get_service, PathRouter, RequestHandlerService};
use picoserve::{response, ResponseSent, Router};

pub type AppRouter = impl PathRouter;

#[define_opaque(AppRouter)]
pub fn make_app(
    data: &'static ValueSynchronizer<MAX_LISTENERS, NoopRawMutex, LightState>,
    logger: &'static RingBufferLogger,
) -> Router<AppRouter> {
    picoserve::Router::new()
        .route(
            "/",
            get_service(response::File::html(include_str!(
                "../resources/index.html"
            ))),
        )
        .route(
            "/ota",
            get_service(response::File::html(include_str!("../resources/ota.html")))
                .post_service(OtaHandler),
        )
        .route("/logs", get_service(LogHandler { logger }))
        .route(
            "/style.css",
            get_service(response::File::css(include_str!("../resources/style.css"))),
        )
        .route(
            "/ws",
            get(move |update: WebSocketUpgrade| update.on_upgrade(ColorHandler { color: data })),
        )
}

pub struct ColorHandler {
    color: &'static ValueSynchronizer<MAX_LISTENERS, NoopRawMutex, LightState>,
}

impl WebSocketCallback for ColorHandler {
    async fn run<R: Read, W: Write<Error = R::Error>>(
        self,
        mut rx: SocketRx<R>,
        mut tx: SocketTx<W>,
    ) -> Result<(), W::Error> {
        let mut message_buffer = [0u8; LIGHT_STATE_LEN];
        let mut watcher = self.color.watch();

        // Send initial message
        log::info!("Websocket opened, sending initial message");
        tx.send_binary(&self.color.read_clone().into_bytes())
            .await?;

        loop {
            match select(rx.next_message(&mut message_buffer), watcher.read()).await {
                Either::First(message) => {
                    let bytes = match message {
                        Ok(Message::Binary(bytes)) => bytes,
                        _ => {
                            log::info!("Websocket closed");
                            return Ok(());
                        }
                    };
                    let Ok(bytes): Result<&[u8; LIGHT_STATE_LEN], _> = bytes.try_into() else {
                        log::info!("Received invalid WS bytes: {bytes:?}");
                        return Ok(());
                    };
                    let message = LightState::from_bytes(bytes);
                    self.color.write(message).await;
                    watcher.skip().await;
                }
                Either::Second(message) => {
                    tx.send_binary(&message.into_bytes()).await?;
                }
            }
        }
    }
}

struct OtaHandler;

impl RequestHandlerService<()> for OtaHandler {
    async fn call_request_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        mut request: Request<'_, R>,
        _response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        let reader = request.body_connection.body().reader();
        log::info!("Starting OTA update...");
        ota_begin(&mut FlashStorage::new(), reader, |_| {})
            .await
            .unwrap();
        log::info!("OTA update finished, resetting...");
        software_reset();
    }
}

struct LogHandler {
    logger: &'static RingBufferLogger,
}

impl RequestHandlerService<()> for LogHandler {
    async fn call_request_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        let logs = self.logger.get_logs();
        let logs = core::str::from_utf8(logs.as_slice()).unwrap();

        logs.write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}
