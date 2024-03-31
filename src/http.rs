use core::future::pending;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Duration;
use embedded_io_async::{Read, Write};
use esp_println::{dbg, println};
use picoserve::routing::{get, PathRouter, post};
use picoserve::*;
use picoserve::io::ErrorKind::InvalidInput;
use picoserve::response::{WebSocketUpgrade, ws};
use picoserve::response::ws::{Message, SocketRx, SocketTx, WebSocketCallback};
use static_cell::make_static;
use crate::wifi::WifiStack;

const PORT: u16 = 80;
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
fn make_app() -> Router<AppRouter> {
    let data: &'static Mutex<NoopRawMutex, ColorData> = make_static!(Mutex::new(ColorData::default()));
    picoserve::Router::new()
        .route("/", get(|| async move{ response::File::html(include_str!("../resources/index.html")) }))
        // .route("/ota", post(|v| todo!()))
        .route("/ws", get(move |update: WebSocketUpgrade| {
            update.on_upgrade(ColorHandler {
                color: data
            })
        }))
}

#[derive(Default)]
pub struct ColorData {
    red: u16,
    blue: u16,
}

pub struct ColorHandler {
    color: &'static Mutex<NoopRawMutex, ColorData>
}

#[derive(Debug)]
pub struct InputMessage {
    save: bool,
    cold: u16,
    warm: u16,
    x: u16,
    y: u16,
}

impl From<&[u8; 10]> for InputMessage {
    fn from(value: &[u8; 10]) -> Self {
        InputMessage {
            save: u16::from_le_bytes([value[0], value[1]]) != 0,
            cold: u16::from_le_bytes([value[2], value[3]]),
            warm: u16::from_le_bytes([value[4], value[5]]),
            x: u16::from_le_bytes([value[6], value[7]]),
            y: u16::from_le_bytes([value[8], value[9]]),
        }
    }
}

impl Into<[u8; 10]> for InputMessage {
    fn into(self) -> [u8; 10] {
        let [c0, c1] = self.cold.to_be_bytes();
        let [w0, w1] = self.warm.to_be_bytes();
        let [x0, x1] = self.x.to_be_bytes();
        let [y0, y1] = self.y.to_be_bytes();
        [0, self.save as u8, c0, c1, w0, w1, x0, x1, y0, y1]
    }
}

impl WebSocketCallback for ColorHandler {
    async fn run<R: Read, W: Write<Error=R::Error>>(self, mut rx: SocketRx<R>, tx: SocketTx<W>) -> Result<(), W::Error> {
        let mut message = [0u8; 16];

        loop {
            let message = rx.next_message(&mut message).await.unwrap();
            let Message::Binary(bytes) = message else {
                println!("Received invalid WS message: {message:?}");
                return Ok(())
            };
            let Ok(bytes): Result<&[u8; 10], _> = bytes.try_into() else {
                println!("Received invalid WS bytes: {bytes:?}");
                return Ok(())
            };
            let msg = InputMessage::from(bytes);
            dbg!(msg);
        }
    }
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
        PORT,
        &mut tcp_rx_buffer,
        &mut tcp_tx_buffer,
        &mut http_buffer,
    ).await
}