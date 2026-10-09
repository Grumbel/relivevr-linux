//! ReliveVR protocol probe + discovery/session responder.
//!
//! Working: discovery Hello, HELLO_DIRECT connect, receive StartRequest + device caps.
//! Next: VideoInit after StartRequest; later binary H.264.

use std::env;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

const DEFAULT_PORT: u16 = 1235;
const FRAG_HEADER_LEN: usize = 15;

/// Live SERVICE_OP_CODE / message types
const TYPE_HELLO: u8 = 0;
const TYPE_START_REQUEST: u8 = 3;
const TYPE_DEVICE_CAPS: u8 = 5;
const TYPE_HELLO_DIRECT: u8 = 7;
/// Guess for VideoInit (not yet confirmed on wire); override with RELIVEVR_VIDEOINIT_TYPE
const TYPE_VIDEO_INIT_DEFAULT: u8 = 2;

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
        buf.extend_from_slice(&length.to_be_bytes());
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

#[derive(Clone, Copy, Debug)]
enum ResponseStyle {
    Minimal,
    Echo,
    Full,
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

fn make_typed_json_packet(seq: u16, type_byte: u8, json: &str) -> Vec<u8> {
    let mut payload = Vec::with_capacity(1 + json.len());
    payload.push(type_byte);
    payload.extend_from_slice(json.as_bytes());
    FragmentHeader::build_single(seq, 0, &payload)
}

/// VideoInit fields from VideoInit::FromJSON: Width, Height, CodecID, NonLinearScaling.
fn make_video_init_json(width: u32, height: u32, codec: &str, nls: bool) -> String {
    format!(
        r#"{{"Width":{width},"Height":{height},"CodecID":"{codec}","NonLinearScaling":{nls}}}"#,
        width = width,
        height = height,
        codec = codec,
        nls = if nls { "true" } else { "false" }
    )
}

/// Best-effort parse of StartRequest fields we care about.
fn parse_start_request(json: &str) -> (u32, u32, String, bool) {
    let mut w = 1440u32;
    let mut h = 1440u32;
    let mut codec = "avc".to_string();
    let mut nls = true;
    if let Some(v) = json.split("\"DisplayWidth\":").nth(1) {
        if let Some(num) = v.split(',').next() {
            if let Ok(n) = num.trim().parse() {
                w = n;
            }
        }
    }
    if let Some(v) = json.split("\"DisplayHeight\":").nth(1) {
        if let Some(num) = v.split(',').next() {
            if let Ok(n) = num.trim().parse() {
                h = n;
            }
        }
    }
    if let Some(v) = json.split("\"VideoCodec\":\"").nth(1) {
        if let Some(s) = v.split('"').next() {
            codec = s.to_string();
        }
    }
    if json.contains("\"NonLinearScalingSupported\":false") {
        nls = false;
    }
    (w, h, codec, nls)
}

fn is_our_payload(payload: &[u8]) -> bool {
    payload.windows(20).any(|w| w == b"relivevr-linux-probe")
}

fn parse_style(s: &str) -> ResponseStyle {
    match s {
        "minimal" => ResponseStyle::Minimal,
        "echo" => ResponseStyle::Echo,
        "full" => ResponseStyle::Full,
        _ => ResponseStyle::Full,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let style = env::var("RELIVEVR_STYLE")
        .map(|s| parse_style(&s))
        .unwrap_or(ResponseStyle::Full);
    let type_byte: u8 = env::var("RELIVEVR_TYPE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(TYPE_HELLO);
    let video_init_type: u8 = env::var("RELIVEVR_VIDEOINIT_TYPE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(TYPE_VIDEO_INIT_DEFAULT);

    let bind_addr: SocketAddr = format!("0.0.0.0:{}", DEFAULT_PORT).parse()?;
    let socket = UdpSocket::bind(bind_addr).await?;
    socket.set_broadcast(true)?;

    info!("Listening on {}", bind_addr);
    info!(
        "Hello style={:?} type={}  VideoInit type={} (RELIVEVR_VIDEOINIT_TYPE)",
        style, type_byte, video_init_type
    );

    let mut buf = vec![0u8; 65535];
    let mut reply_seq: u16 = 1;
    let mut announce_seq: u16 = 1;

    let mut announce = time::interval(Duration::from_secs(2));
    announce.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = announce.tick() => {
                let json = make_hello_json(style);
                let packet = make_typed_json_packet(announce_seq, type_byte, &json);
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

                        let msg_type = payload[0];
                        info!(
                            "from {} seq={} field2={} off={} len={} flags=0x{:02x} pkt={}",
                            src, hdr.seq, hdr.field2, hdr.offset, hdr.length, hdr.flags, len
                        );

                        if payload.len() > 1 && (payload[1] == b'{' || payload[1] == b'[') {
                            if let Ok(s) = std::str::from_utf8(&payload[1..]) {
                                let s = s.trim_end_matches('\0');
                                info!("  type={} JSON: {}", msg_type, s);

                                match msg_type {
                                    TYPE_HELLO | TYPE_HELLO_DIRECT => {
                                        let resp_type = if env::var("RELIVEVR_TYPE").is_ok() {
                                            type_byte
                                        } else {
                                            TYPE_HELLO
                                        };
                                        let resp_style = if msg_type == TYPE_HELLO_DIRECT {
                                            ResponseStyle::Full
                                        } else {
                                            style
                                        };
                                        let json = make_hello_json(resp_style);
                                        let packet = make_typed_json_packet(reply_seq, resp_type, &json);
                                        reply_seq = reply_seq.wrapping_add(1);
                                        match socket.send_to(&packet, src).await {
                                            Ok(n) => info!(
                                                "  -> HelloResponse {}B style={:?} type={} (req {}) -> {}",
                                                n, resp_style, resp_type, msg_type, src
                                            ),
                                            Err(e) => warn!("  -> reply failed: {}", e),
                                        }
                                        // Decoder is created ~30ms after connect with empty MIME
                                        // (video/ + ""). Push VideoInit early; StartRequest refines.
                                        if msg_type == TYPE_HELLO_DIRECT {
                                            let vij = make_video_init_json(1440, 1440, "avc", true);
                                            let vp = make_typed_json_packet(reply_seq, video_init_type, &vij);
                                            reply_seq = reply_seq.wrapping_add(1);
                                            match socket.send_to(&vp, src).await {
                                                Ok(n) => info!(
                                                    "  -> early VideoInit {}B type={} -> {}",
                                                    n, video_init_type, src
                                                ),
                                                Err(e) => warn!("  -> early VideoInit failed: {}", e),
                                            }
                                        }
                                    }
                                    TYPE_START_REQUEST => {
                                        let (w, h, codec, nls) = parse_start_request(s);
                                        info!(
                                            "  StartRequest: {}x{} codec={} nls={}",
                                            w, h, codec, nls
                                        );
                                        // Type byte for VideoInit still unconfirmed.
                                        // Try configured type first, then common alternates.
                                        // CodecID: client builds video/+CodecID; also try full MIME.
                                        let mut types = vec![video_init_type];
                                        for t in [2u8, 4, 8, 9, 10] {
                                            if !types.contains(&t) {
                                                types.push(t);
                                            }
                                        }
                                        let codec_variants = [
                                            codec.clone(),
                                            format!("video/{}", codec),
                                            "avc".to_string(),
                                            "video/avc".to_string(),
                                        ];
                                        for &t in &types {
                                            for c in &codec_variants {
                                                let vij = make_video_init_json(w, h, c, nls);
                                                let packet = make_typed_json_packet(reply_seq, t, &vij);
                                                reply_seq = reply_seq.wrapping_add(1);
                                                match socket.send_to(&packet, src).await {
                                                    Ok(n) => info!(
                                                        "  -> VideoInit {}B type={} codec={} -> {}",
                                                        n, t, c, src
                                                    ),
                                                    Err(e) => warn!("  -> VideoInit failed: {}", e),
                                                }
                                            }
                                        }
                                    }
                                    TYPE_DEVICE_CAPS => {
                                        info!("  device caps (no reply yet)");
                                    }
                                    other => {
                                        info!("  unhandled type {}", other);
                                    }
                                }
                            } else {
                                info!("  type={} hex: {}", msg_type, hex_preview(payload, 64));
                            }
                        } else {
                            info!("  type={} hex: {}", msg_type, hex_preview(payload, 64));
                        }
                    }
                    Err(e) => warn!("recv error: {}", e),
                }
            }
        }
    }
}
