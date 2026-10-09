//! ReliveVR protocol probe + discovery responder.
//!
//! Live HelloRequest (VR-1541F):
//!   type=0 {"DeviceID":"…","MaxDatagramSize":65507,
//!           "Options":{"DeviceType":{"Type":"string","Val":"VR-1541F"}},
//!           "ProtocolMinVersion":1,"ProtocolVersion":1}

use std::env;
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

    fn build_single(seq: u16, flags: u8, payload: &[u8]) -> Vec<u8> {
        let length = payload.len() as u32;
        let mut buf = Vec::with_capacity(FRAG_HEADER_LEN + payload.len());
        buf.extend_from_slice(&seq.to_be_bytes());
        buf.extend_from_slice(&length.to_be_bytes()); // field2 = payload len (live)
        buf.extend_from_slice(&0u32.to_be_bytes());
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

/// Response variants to try when the client rejects/crashes on Hello.
#[derive(Clone, Copy, Debug)]
enum ResponseStyle {
    /// Minimal: only fields the client itself sends, plus ServerName.
    Minimal,
    /// Full: ChannelsSupported + Transports + Options.
    Full,
    /// Echo-like: same keys as request, no ChannelsSupported/Transports.
    Echo,
}

fn make_hello_json(style: ResponseStyle) -> String {
    match style {
        ResponseStyle::Minimal => {
            r#"{"ProtocolVersion":1,"ProtocolMinVersion":1,"MaxDatagramSize":65507,"DatagramSize":65507,"Port":1235,"DeviceID":"relivevr-linux-probe","ServerName":"ReliveVR Linux Probe"}"#.to_string()
        }
        ResponseStyle::Echo => {
            r#"{"DeviceID":"relivevr-linux-probe","MaxDatagramSize":65507,"DatagramSize":65507,"Port":1235,"Options":{"DeviceType":{"Type":"string","Val":"PC"}},"ProtocolMinVersion":1,"ProtocolVersion":1,"ServerName":"ReliveVR Linux Probe"}"#.to_string()
        }
        ResponseStyle::Full => {
            r#"{"ProtocolVersion":1,"ProtocolMinVersion":1,"MaxDatagramSize":65507,"DatagramSize":65507,"Port":1235,"DeviceID":"relivevr-linux-probe","Options":{"DeviceType":{"Type":"string","Val":"PC"}},"ServerName":"ReliveVR Linux Probe","ChannelsSupported":[true,true,true,true,true,true,true,true],"Transports":["UDP"]}"#.to_string()
        }
    }
}

fn make_hello_packet(seq: u16, style: ResponseStyle, type_byte: u8) -> Vec<u8> {
    let json = make_hello_json(style);
    let mut payload = Vec::with_capacity(1 + json.len());
    payload.push(type_byte);
    payload.extend_from_slice(json.as_bytes());
    FragmentHeader::build_single(seq, 0, &payload)
}

fn is_our_payload(payload: &[u8]) -> bool {
    payload.windows(20).any(|w| w == b"relivevr-linux-probe")
}

fn parse_style(s: &str) -> ResponseStyle {
    match s {
        "minimal" => ResponseStyle::Minimal,
        "echo" => ResponseStyle::Echo,
        "full" => ResponseStyle::Full,
        _ => ResponseStyle::Minimal,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // RELIVEVR_STYLE=minimal|echo|full  RELIVEVR_TYPE=0|1
    let style = env::var("RELIVEVR_STYLE")
        .map(|s| parse_style(&s))
        .unwrap_or(ResponseStyle::Minimal);
    let type_byte: u8 = env::var("RELIVEVR_TYPE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let bind_addr: SocketAddr = format!("0.0.0.0:{}", DEFAULT_PORT).parse()?;
    let socket = UdpSocket::bind(bind_addr).await?;
    socket.set_broadcast(true)?;

    info!("Listening on {}", bind_addr);
    info!("Response style={:?} type_byte={} (override with RELIVEVR_STYLE / RELIVEVR_TYPE)", style, type_byte);

    let mut buf = vec![0u8; 65535];
    let mut reply_seq: u16 = 1;
    let mut announce_seq: u16 = 1;

    let mut announce = time::interval(Duration::from_secs(2));
    announce.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = announce.tick() => {
                let packet = make_hello_packet(announce_seq, style, type_byte);
                announce_seq = announce_seq.wrapping_add(1);
                let bcast = SocketAddr::from((Ipv4Addr::BROADCAST, DEFAULT_PORT));
                let _ = socket.send_to(&packet, bcast).await;
            }
            recv = socket.recv_from(&mut buf) => {
                match recv {
                    Ok((len, src)) => {
                        let data = &buf[..len];
                        let hdr = match FragmentHeader::parse(data) {
                            Some(h) => h,
                            None => {
                                info!("from {} len={} no-hdr {}", src, len, hex_preview(data, 32));
                                continue;
                            }
                        };
                        if len <= FRAG_HEADER_LEN {
                            continue;
                        }
                        let payload = &data[FRAG_HEADER_LEN..];
                        if is_our_payload(payload) {
                            continue;
                        }

                        info!(
                            "from {} seq={} field2={} off={} len={} flags=0x{:02x} pkt={}",
                            src, hdr.seq, hdr.field2, hdr.offset, hdr.length, hdr.flags, len
                        );
                        // Any type whose body looks like JSON
                        if payload.len() > 1 && (payload[1] == b'{' || payload[1] == b'[') {
                            if let Ok(s) = std::str::from_utf8(&payload[1..]) {
                                info!("  type={} JSON: {}", payload[0], s.trim_end_matches('\0'));
                            } else {
                                info!("  type={} hex: {}", payload[0], hex_preview(payload, 64));
                            }
                        } else {
                            info!("  type={} hex: {}", payload[0], hex_preview(payload, 64));
                        }

                        // Reply to discovery-family types (0 = HelloRequest, 7 = seen live after our reply)
                        let t = payload[0];
                        if matches!(t, 0 | 1 | 7) && hdr.offset == 0 {
                            // type 7 = SERVICE_OP_CODE_HELLO_DIRECT (from client logcat).
                            // Discovery (0) can use Minimal; HELLO_DIRECT needs a fuller
                            // ServerParameters-capable HelloResponse. Response type defaults
                            // to 0 unless RELIVEVR_TYPE is set (try 0 first for DIRECT).
                            let resp_type = if std::env::var("RELIVEVR_TYPE").is_ok() {
                                type_byte
                            } else {
                                0
                            };
                            let resp_style = if t == 7 && std::env::var("RELIVEVR_STYLE").is_err() {
                                ResponseStyle::Full
                            } else {
                                style
                            };
                            let packet = make_hello_packet(reply_seq, resp_style, resp_type);
                            reply_seq = reply_seq.wrapping_add(1);
                            match socket.send_to(&packet, src).await {
                                Ok(n) => info!(
                                    "  -> reply {} bytes style={:?} type={} (req type={}) -> {}",
                                    n, resp_style, resp_type, t, src
                                ),
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
