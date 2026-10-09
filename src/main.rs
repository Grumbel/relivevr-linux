//! ReliveVR protocol probe.
//!
//! Listens on UDP 1235, parses FlowCtrlProtocol fragment headers
//! (reverse-engineered), and when the payload looks like a control
//! message (type byte + JSON) prints the JSON.

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
    field2: u32,
    offset: u32,
    length: u32,
    flags: u8,
}

impl FragmentHeader {
    fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < FRAG_HEADER_LEN {
            return None;
        }
        Some(Self {
            seq: u16::from_be_bytes([buf[0], buf[1]]),
            field2: u32::from_be_bytes([buf[2], buf[3], buf[4], buf[5]]),
            offset: u32::from_be_bytes([buf[6], buf[7], buf[8], buf[9]]),
            length: u32::from_be_bytes([buf[10], buf[11], buf[12], buf[13]]),
            flags: buf[14],
        })
    }
}

fn try_print_control_payload(payload: &[u8]) {
    if payload.is_empty() {
        return;
    }
    let msg_type = payload[0];
    let body = &payload[1..];
    // Heuristic: if it looks like JSON, print it.
    let is_jsonish = body.starts_with(b"{") || body.starts_with(b"[");
    if is_jsonish {
        match std::str::from_utf8(body) {
            Ok(s) => {
                // Trim trailing NULs if present
                let s = s.trim_end_matches('\0');
                info!("  control type={} JSON: {}", msg_type, s);
            }
            Err(_) => {
                info!(
                    "  control type={} (non-utf8 JSON-like) {:02x?}",
                    msg_type,
                    &body[..std::cmp::min(body.len(), 64)]
                );
            }
        }
    } else {
        info!(
            "  payload type={} (binary or other) first32: {:02x?}",
            msg_type,
            &payload[..std::cmp::min(payload.len(), 32)]
        );
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
    info!("Fragment header = 15 bytes (BE); control plane after type byte is JSON");

    let mut buf = vec![0u8; 65535];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                let data = &buf[..len];
                if let Some(hdr) = FragmentHeader::parse(data) {
                    let expected = FRAG_HEADER_LEN as u32 + hdr.length;
                    let ok = len as u32 == expected;
                    info!(
                        "from {}  seq={} field2={} off={} len={} flags=0x{:02x}  pkt={} exp={} {}",
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
                        try_print_control_payload(payload);
                    }
                } else {
                    info!(
                        "from {}  (too short) {} bytes: {:02x?}",
                        src,
                        len,
                        &data[..std::cmp::min(len, 64)]
                    );
                }
            }
            Err(e) => warn!("recv error: {}", e),
        }
    }
}
