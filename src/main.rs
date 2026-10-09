//! Minimal ReliveVR discovery / protocol probe.
//!
//! Listens on UDP 1235, parses FlowCtrlProtocol fragment headers
//! (reverse-engineered from libwirelessvr-lib.so), and logs them.

use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

const DEFAULT_PORT: u16 = 1235;
const FRAG_HEADER_LEN: usize = 15;

/// Reverse-engineered 15-byte fragment header (big-endian multi-byte fields).
#[derive(Debug, Clone, Copy)]
struct FragmentHeader {
    seq: u16,
    field2: u32,   // likely total message size or message ID related
    offset: u32,   // offset of this fragment within the full message
    length: u32,   // payload length of this fragment
    flags: u8,
}

impl FragmentHeader {
    fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < FRAG_HEADER_LEN {
            return None;
        }
        let seq = u16::from_be_bytes([buf[0], buf[1]]);
        let field2 = u32::from_be_bytes([buf[2], buf[3], buf[4], buf[5]]);
        let offset = u32::from_be_bytes([buf[6], buf[7], buf[8], buf[9]]);
        let length = u32::from_be_bytes([buf[10], buf[11], buf[12], buf[13]]);
        let flags = buf[14];
        Some(Self {
            seq,
            field2,
            offset,
            length,
            flags,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let bind_addr: SocketAddr = format!("0.0.0.0:{}", DEFAULT_PORT).parse()?;
    let socket = UdpSocket::bind(bind_addr).await?;
    info!("Listening for ReliveVR traffic on {}", bind_addr);
    info!("Fragment header is 15 bytes, multi-byte fields big-endian");

    let mut buf = vec![0u8; 65535];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                let data = &buf[..len];
                if let Some(hdr) = FragmentHeader::parse(data) {
                    let expected = FRAG_HEADER_LEN as u32 + hdr.length;
                    let ok = len as u32 == expected;
                    info!(
                        "from {}  seq={} field2={} off={} len={} flags=0x{:02x}  pkt_len={} expected={} {}",
                        src,
                        hdr.seq,
                        hdr.field2,
                        hdr.offset,
                        hdr.length,
                        hdr.flags,
                        len,
                        expected,
                        if ok { "OK" } else { "SIZE MISMATCH" }
                    );
                    if len > FRAG_HEADER_LEN {
                        let payload = &data[FRAG_HEADER_LEN..];
                        let preview = &payload[..std::cmp::min(payload.len(), 32)];
                        info!("  payload[0..{}]: {:02x?}", preview.len(), preview);
                    }
                } else {
                    info!(
                        "from {}  (too short for fragment header) {} bytes: {:02x?}",
                        src,
                        len,
                        &data[..std::cmp::min(len, 64)]
                    );
                }
            }
            Err(e) => {
                warn!("recv error: {}", e);
            }
        }
    }
}
