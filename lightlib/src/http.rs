use embassy_executor::{SpawnToken, Spawner};
use embassy_net::Stack;
use embassy_time::Duration;
use picoserve::make_static;
use picoserve::routing::PathRouter;
use picoserve::*;

pub const PORT: u16 = 80;
pub const MAX_CONNECTIONS: usize = 8;
pub const MAX_LISTENERS: usize = MAX_CONNECTIONS + 4;

pub async fn setup_http_server<S, A: PathRouter>(
    stack: Stack<'static>,
    spawner: Spawner,
    app: &'static Router<A>,
    web_task: fn(
        usize,
        Stack<'static>,
        &'static Router<A>,
        &'static Config<Duration>,
    ) -> SpawnToken<S>,
) {
    let config = make_static!(
        Config<Duration>,
        Config::new(Timeouts {
            start_read_request: None,
            persistent_start_read_request: None,
            read_request: None,
            write: None,
        })
    );

    for id in 0..MAX_CONNECTIONS {
        spawner.must_spawn::<S>(web_task(id, stack, app, config));
    }
}
