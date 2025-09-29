use core::net::{IpAddr, SocketAddr};
use embassy_net::Stack;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use sntpc::{get_time, NtpContext, NtpTimestampGenerator};
use embassy_time::{Duration, Instant, Timer};
use embassy_net::dns::DnsQueryType;

const NTP_SERVER: &str = "pool.ntp.org";

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

pub async fn init_ntp(stack: Stack<'_>) {
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
    socket.bind(123).unwrap();

    let context = NtpContext::new(Timestamp::default());

    let Ok(ntp_addrs) = stack
        .dns_query(NTP_SERVER, DnsQueryType::A)
        .await
    else {
        log::info!("Failed to resolve DNS");
        return;
    };

    if ntp_addrs.is_empty() {
        log::info!("Failed to resolve DNS");
        return;
    }

    let addr: IpAddr = ntp_addrs[0].into();
    let result =
        get_time(SocketAddr::from((addr, 123)), &socket, context)
            .await;

    match result {
        Ok(time) => {
            log::info!("Time: {:?}", time);
        }
        Err(e) => {
            log::info!("Error getting time: {:?}", e);
        }
    }

    //Timer::after(Duration::from_secs(15)).await;
}