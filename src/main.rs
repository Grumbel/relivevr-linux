//! ReliveVR protocol probe + discovery responder.
//!
//! Observed client HelloRequest (192.168.178.33, VR-1541F):
//!   {"DeviceID":"4b94589deb5e1561","MaxDatagramSize":65507,
//!    "Options":{"DeviceType":{"Type":"string","Val":"VR-1541F"}},
//!    "ProtocolMinVersion":1,"ProtocolVersion":1}
//!
//! Fragment header: field2 == payload length for single-fragment msgs
//! (not total packet size). seq=0 on client discovery probes.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

const DEFAULT_PORT: u16 = 1235;
const FRAG_HEADER_LEN: usize = 15;

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

    /// Single-fragment packet. field2 matches observed client behaviour (= payload length).
    fn build_single(seq: u16, flags: u8, payload: &[u8]) -> Vec<u8> {
        let length = payload.len() as u32;
        let mut buf = Vec::with_capacity(FRAG_HEADER_LEN + payload.len());
        buf.extend_from_slice(&seq.to_be_bytes());
        buf.extend_from_slice(&length.to_be_bytes()); // field2 = payload length (observed)
        buf.extend_from_slice(&0u32.to_be_bytes());   // offset 0
        buf.extend_from_slice(&length.to_be_bytes());
        buf.push(flags);
        buf.extend_from_slice(payload);
        buf
    }
}

fn hex_preview(data: &[u8], max: usize) -> String {
    data[..std::cmp::min(data.len(), max)]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn try_print_control_payload(payload: &[u8]) {
    if payload.is_empty() {
        return;
    }
    let msg_type = payload[0];
    let body = &payload[1..];
    if body.starts_with(b"{") || body.starts_with(b"[") {
        match std::str::from_utf8(body) {
            Ok(s) => info!("  type={} JSON: {}", msg_type, s.trim_end_matches('\0')),
            Err(_) => info!("  type={} non-utf8: {}", msg_type, hex_preview(body, 80)),
        }
    } else {
        info!("  type={} hex: {}", msg_type, hex_preview(payload, 64));
    }
}

/// HelloResponse shaped closer to what FromJSON expects + client Options style.
fn make_hello_response_json() -> String {
    // ChannelsSupported: client table is 8 bools; send all true.
    // Options: use AMF-variant-like object similar to the client's DeviceType.
    // Transports: UDP only for now.
    r#"{"ProtocolVersion":1,"ProtocolMinVersion":1,"MaxDatagramSize":65507,"DeviceID":"relivevr-linux-probe","Options":{"DeviceType":{"Type":"string","Val":"PC"}},"ServerName":"ReliveVR Linux Probe","ChannelsSupported":[true,true,true,true,true,true,true,true],"Transports":["UDP"]}"#.to_string()
}

fn make_hello_packet(seq: u16) -> Vec<u8> {
    let json = make_hello_response_json();
    let mut payload = Vec::with_capacity(1 + json.len());
    payload.push(0u8);
    payload.extend_from_slice(json.as_bytes());
    FragmentHeader::build_single(seq, 0, &payload)
}

fn is_our_payload(payload: &[u8]) -> bool {
    payload.windows(20).any(|w| w == b"relivevr-linux-probe")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let bind_addr: SocketAddr = format!("0.0.0.0:{}", DEFAULT_PORT).parse()?;
    let socket = UdpSocket::bind(bind_addr).await?;
    socket.set_broadcast(true)?;

    info!("Listening on {}", bind_addr);
    info!("Replying to type-0 HelloRequest; announcing every 2s");

    let mut buf = vec![0u8; 65535];
    let mut reply_seq: u16 = 1;
    let mut announce_seq: u16 = 1;

    let mut announce = time::interval(Duration::from_secs(2));
    announce.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = announce.tick() => {
                let packet = make_hello_packet(announce_seq);
                announce_seq = announce_seq.wrapping_add(1);
                let bcast = SocketAddr::from((Ipv4Addr::BROADCAST, DEFAULT_PORT));
                if let Err(e) = socket.send_to(&packet, bcast).await {
                    warn!("announce failed: {}", e);
                }
            }
            recv = socket.recv_from(&mut buf) => {
                match recv {
                    Ok((len, src)) => {
                        let data = &buf[..len];

                        let hdr = match FragmentHeader::parse(data) {
                            Some(h) => h,
                            None => {
                                info!("from {} len={} (no frag hdr) {}", src, len, hex_preview(data, 32));
                                continue;
                            }
                        };

                        if len <= FRAG_HEADER_LEN {
                            continue;
                        }
                        let payload = &data[FRAG_HEADER_LEN..];

                        // Drop our own loopback announces
                        if is_our_payload(payload) {
                            continue;
                        }

                        let expected = FRAG_HEADER_LEN as u32 + hdr.length;
                        info!(
                            "from {} seq={} field2={} off={} len={} flags=0x{:02x} pkt={} exp={} {}",
                            src, hdr.seq, hdr.field2, hdr.offset, hdr.length, hdr.flags,
                            len, expected, if len as u32 == expected { "OK" } else { "MISMATCH" }
                        );
                        try_print_control_payload(payload);

                        // Reply to discovery / HelloRequest (type 0, single fragment)
                        if payload.first() == Some(&0) && hdr.offset == 0 {
                            let packet = make_hello_packet(reply_seq);
                            reply_seq = reply_seq.wrapping_add(1);
                            match socket.send_to(&packet, src).await {
                                Ok(n) => info!("  -> HelloResponse {} bytes -> {}", n, src),
                                Err(e) => warn!("  -> reply failed: {}", e),
                            }
                        }
                    }
                    Err(e) => warn!("recv error: {}", e),
                }
            }
        }
    }
}
