use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embedded_io_async::{Read, Write};
use esp_println::println;
use picoserve::{response, Router};
use picoserve::response::WebSocketUpgrade;
use picoserve::response::ws::{Message, SocketRx, SocketTx, WebSocketCallback};
use picoserve::routing::{get, PathRouter};
use static_cell::make_static;
use crate::value_synchronizer::ValueSynchronizer;
use crate::http::MAX_CONNECTIONS;

pub type AppRouter = impl PathRouter;
pub fn make_app() -> Router<AppRouter> {
    let data: &'static _ = make_static!(ValueSynchronizer::new(InputMessage::default()));
    picoserve::Router::new()
        .route("/", get(|| async move{ response::File::html(include_str!("../resources/index.html")) }))
        // .route("/ota", get(|v| todo!()))
        // .route("/ota", post(|v| todo!()))
        .route("/ws", get(move |update: WebSocketUpgrade| {
            update.on_upgrade(ColorHandler {
                color: data,
            })
        }))
}

pub struct ColorHandler {
    color: &'static ValueSynchronizer<MAX_CONNECTIONS, NoopRawMutex, InputMessage>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct InputMessage {
    save: bool,
    cold: u16,
    warm: u16,
    x: u16,
    y: u16,
}

impl InputMessage {
    pub fn from_bytes(bytes: &[u8; 10]) -> Self {
        Self {
            save: u16::from_le_bytes([bytes[0], bytes[1]]) != 0,
            cold: u16::from_le_bytes([bytes[2], bytes[3]]),
            warm: u16::from_le_bytes([bytes[4], bytes[5]]),
            x: u16::from_le_bytes([bytes[6], bytes[7]]),
            y: u16::from_le_bytes([bytes[8], bytes[9]]),
        }
    }

    pub fn into_bytes(self) -> [u8; 10] {
        let [c0, c1] = self.cold.to_le_bytes();
        let [w0, w1] = self.warm.to_le_bytes();
        let [x0, x1] = self.x.to_le_bytes();
        let [y0, y1] = self.y.to_le_bytes();
        [0, 0, c0, c1, w0, w1, x0, x1, y0, y1]
    }
}

impl WebSocketCallback for ColorHandler {
    async fn run<R: Read, W: Write<Error=R::Error>>(self, mut rx: SocketRx<R>, mut tx: SocketTx<W>) -> Result<(), W::Error> {
        let mut message_buffer = [0u8; 16];
        let mut watcher = self.color.watch();

        // Send initial message
        println!("Websocket opened, sending initial message");
        tx.send_binary(&self.color.read_clone().into_bytes()).await?;

        loop {
            match select(
                rx.next_message(&mut message_buffer),
                watcher.read(),
            ).await {
                Either::First(message) => {
                    let bytes = match message.unwrap() {
                        Message::Binary(bytes) => bytes,
                        Message::Close(_) => {
                            println!("Websocket closed");
                            return Ok(())
                        },
                        message => {
                            println!("Received invalid WS message: {message:?}");
                            return Ok(())
                        },
                    };
                    let Ok(bytes): Result<&[u8; 10], _> = bytes.try_into() else {
                        println!("Received invalid WS bytes: {bytes:?}");
                        return Ok(())
                    };
                    let message = InputMessage::from_bytes(bytes);
                    println!("Received message: {message:?}");
                    self.color.write(message).await;
                    watcher.skip().await;
                }
                Either::Second(message) => {
                    println!("Resending message: {message:?}");
                    tx.send_binary(&message.into_bytes()).await?;
                }
            }
        }
    }
}
