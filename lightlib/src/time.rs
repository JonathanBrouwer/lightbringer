use alloc::string::ToString;
use core::future::Future;
use core::net::{IpAddr, SocketAddr};
use chrono::TimeZone;
use embassy_net::Stack;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use sntpc::{get_time, NtpContext, NtpTimestampGenerator};
use embassy_time::{Duration, Instant, Timer, WithTimeout};
use embassy_net::dns::DnsQueryType;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use chrono_tz::Europe::Amsterdam;

const NTP_SERVER: &str = "pool.ntp.org";

/// Number of microseconds between UNIX time and machine startup
static NTP_OFFSET: Mutex<CriticalSectionRawMutex, Option<i64>> = Mutex::new(None);

#[derive(Copy, Clone, Default)]
struct Timestamp {
    duration: Duration,
}

impl NtpTimestampGenerator for Timestamp {
    fn init(&mut self) {
        self.duration = Instant::now().duration_since(Instant::from_ticks(0))
    }

    fn timestamp_sec(&self) -> u64 {
        self.duration.as_secs()
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        (self.duration.as_micros() % 1_000_000) as u32
    }
}

/// Returns current UNIX timestamp in microseconds
///
/// Returns None if ntp is not yet available
async fn now_micros() -> Option<i64> {
    let ntp_offset = (*NTP_OFFSET.lock().await)?.clone();
    Some(Instant::now().as_micros() as i64 + ntp_offset)
}

async fn get_ntp_ip(stack: Stack<'_>) -> Result<IpAddr, embassy_net::dns::Error> {
    let ntp_addrs = stack
        .dns_query(NTP_SERVER, DnsQueryType::A)
        .await?;

    ntp_addrs.get(0)
        .and_then(|a| Some(IpAddr::from(*a)))
        .ok_or(embassy_net::dns::Error::Failed)
}

/// Task that periodically gets the current time via NTP
#[embassy_executor::task]
pub async fn ntp_task(stack: Stack<'static>) -> ! {
    'outer: loop {
        log::info!("starting NTP task");

        let Ok(addr) = get_ntp_ip(stack).await else {
            log::warn!("Could not resolve NTP IP");
            Timer::after(Duration::from_secs(5)).await;
            continue 'outer;
        };

        let context = NtpContext::new(Timestamp::default());

        'inner: loop {
            // Create UDP socket
            let mut rx_meta = [PacketMetadata::EMPTY; 16];
            let mut rx_buffer = [0; 4096];
            let mut tx_meta = [PacketMetadata::EMPTY; 16];
            let mut tx_buffer = [0; 4096];

            let mut socket = UdpSocket::new(
                stack,
                &mut rx_meta,
                &mut rx_buffer,
                &mut tx_meta,
                &mut tx_buffer,
            );

            if socket.bind(123).is_err() {
                log::warn!("Could not bind to NTP port");
                Timer::after(Duration::from_secs(5)).await;
                break;
            };

            let result =
                get_time(SocketAddr::from((addr, 123)), &socket, context)
                    .with_timeout(Duration::from_secs(5)).await;

            let result = if result.is_err() {
                log::warn!("Timed out waiting for NTP result");
                break;
            } else {
                result.unwrap()
            };

            match result {
                Ok(time) => {
                    *NTP_OFFSET.lock().await = Some(time.offset);
                    let local_time = Amsterdam.timestamp_micros(now_micros().await.unwrap());
                    log::info!("Time: {}", local_time.earliest().unwrap().to_string());
                }
                Err(e) => {
                    log::warn!("Error getting time: {:?}", e);
                    Timer::after(Duration::from_secs(5)).await;
                    break;
                }
            }

            Timer::after(Duration::from_secs(15)).await;
        }
    }
}