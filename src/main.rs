//! ReliveVR protocol probe + minimal discovery responder.
//!
//! Listens on UDP 1235, parses FlowCtrlProtocol fragment headers,
//! prints JSON control messages, and replies to type-0 discovery
//! probes with a crafted HelloResponse.

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

    /// Build a single-fragment packet for a complete payload.
    fn build_single(seq: u16, flags: u8, payload: &[u8]) -> Vec<u8> {
        let length = payload.len() as u32;
        let mut buf = Vec::with_capacity(FRAG_HEADER_LEN + payload.len());
        buf.extend_from_slice(&seq.to_be_bytes());
        // field2: using total size (header+payload) as a reasonable guess
        let total = (FRAG_HEADER_LEN as u32) + length;
        buf.extend_from_slice(&total.to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes()); // offset 0
        buf.extend_from_slice(&length.to_be_bytes());
        buf.push(flags);
        buf.extend_from_slice(payload);
        buf
    }
}

fn try_print_control_payload(payload: &[u8]) {
    if payload.is_empty() {
        return;
    }
    let msg_type = payload[0];
    let body = &payload[1..];
    let is_jsonish = body.starts_with(b"{") || body.starts_with(b"[");
    if is_jsonish {
        match std::str::from_utf8(body) {
            Ok(s) => {
                let s = s.trim_end_matches('\0');
                info!("  control type={} JSON: {}", msg_type, s);
            }
            Err(_) => {
                info!(
                    "  control type={} (non-utf8) {:02x?}",
                    msg_type,
                    &body[..std::cmp::min(body.len(), 64)]
                );
            }
        }
    } else {
        info!(
            "  payload type={} first32: {:02x?}",
            msg_type,
            &payload[..std::cmp::min(payload.len(), 32)]
        );
    }
}

/// Craft a minimal HelloResponse JSON that a client might accept.
/// Keys recovered from HelloResponse::FromJSON + rodata.
fn make_hello_response_json() -> String {
    // Plausible values; ProtocolVersion / MinVersion are currently guesses.
    // ChannelsSupported is an array of bools (up to 8 observed in disassembly).
    r#"{
  "ProtocolVersion": 1,
  "ProtocolMinVersion": 1,
  "MaxDatagramSize": 65507,
  "DeviceID": "relivevr-linux-probe",
  "Options": 0,
  "ServerName": "ReliveVR Linux Probe",
  "ChannelsSupported": [true, true, true, true, true, true, true, true],
  "Transports": ["UDP"]
}"#
    .replace('\n', "")
    .replace("  ", "")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let bind_addr: SocketAddr = format!("0.0.0.0:{}", DEFAULT_PORT).parse()?;
    let socket = UdpSocket::bind(bind_addr).await?;
    // Enable broadcast replies if needed
    socket.set_broadcast(true)?;
    info!("Listening for ReliveVR traffic on {}", bind_addr);
    info!("Will reply to type-0 discovery probes with a HelloResponse");

    let mut buf = vec![0u8; 65535];
    let mut reply_seq: u16 = 1;

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

                        // Type 0 + single fragment → treat as discovery request and reply
                        if payload.first() == Some(&0) && hdr.offset == 0 {
                            let json = make_hello_response_json();
                            // type=0 (or 1?) + JSON. Using type 0 for the response as well for now.
                            let mut resp_payload = Vec::with_capacity(1 + json.len());
                            resp_payload.push(0u8);
                            resp_payload.extend_from_slice(json.as_bytes());

                            let packet = FragmentHeader::build_single(
                                reply_seq,
                                hdr.flags, // echo flags for now
                                &resp_payload,
                            );
                            reply_seq = reply_seq.wrapping_add(1);

                            match socket.send_to(&packet, src).await {
                                Ok(n) => info!("  -> sent HelloResponse ({} bytes) to {}", n, src),
                                Err(e) => warn!("  -> failed to send reply: {}", e),
                            }
                        }
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
