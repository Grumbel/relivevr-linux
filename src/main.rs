//! Minimal ReliveVR discovery / protocol probe.
//!
//! Listens on UDP 1235, logs every datagram, and (later) will reply to
//! discovery probes and inject video.

use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

const DEFAULT_PORT: u16 = 1235;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let bind_addr: SocketAddr = format!("0.0.0.0:{}", DEFAULT_PORT).parse()?;
    let socket = UdpSocket::bind(bind_addr).await?;
    info!("Listening for ReliveVR traffic on {}", bind_addr);

    let mut buf = vec![0u8; 65535];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                let data = &buf[..len];
                info!(
                    "recv {} bytes from {}: {:02x?}",
                    len,
                    src,
                    &data[..std::cmp::min(len, 64)]
                );
                // TODO: parse discovery, reply, etc.
            }
            Err(e) => {
                warn!("recv error: {}", e);
            }
        }
    }
}
