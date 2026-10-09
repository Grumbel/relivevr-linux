//! Software H.264 encode of RGBA frames (OpenH264) for the ReliveVR video channel.
//!
//! Produces Annex-B NAL units suitable for `make_video_frame_packet`.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use openh264::encoder::{Encoder, EncoderConfig};
use openh264::formats::YUVSource;
use openh264::OpenH264API;
use tracing::info;

/// Default encode size (balance quality vs CPU). VideoInit uses the same when live.
pub const ENCODE_W: u32 = 512;
pub const ENCODE_H: u32 = 512;

/// Latest encoded access unit shared between the GL thread and the UDP server.
#[derive(Clone, Default)]
pub struct LiveVideo {
    /// Annex-B NALs — left eye (frmType 0).
    pub left: Vec<u8>,
    /// Annex-B NALs — right eye (frmType 1).
    pub right: Vec<u8>,
    pub is_idr: bool,
    pub pts_us: u64,
    pub frame_index: u64,
    /// SPS+PPS extracted from the first left IDR (for VideoInit trailer).
    pub param_sets: Vec<u8>,
}

pub type LiveVideoSlot = Arc<Mutex<LiveVideo>>;

pub fn new_slot() -> LiveVideoSlot {
    Arc::new(Mutex::new(LiveVideo::default()))
}

/// OpenH264 wrapper; call `encode_rgba` from the GL thread after readback.
pub struct H264Encoder {
    enc: Encoder,
    width: u32,
    height: u32,
    frame_index: u64,
    origin: Instant,
    force_idr_every: u64,
}

impl H264Encoder {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        // openh264 0.6: resolution comes from the YUV frame; config is bitrate/fps.
        // 512² stereo ×2 is much lighter than 720²; bitrate is per-eye.
        let cfg = EncoderConfig::new()
            .set_bitrate_bps(4_000_000)
            .max_frame_rate(24.0)
            .enable_skip_frame(false);
        let api = OpenH264API::from_source();
        let enc = Encoder::with_api_config(api, cfg)
            .map_err(|e| format!("OpenH264 init: {e:?}"))?;
        info!("OpenH264 encoder ready {width}x{height}");
        Ok(Self {
            enc,
            width,
            height,
            frame_index: 0,
            origin: Instant::now(),
            force_idr_every: 24,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }

    /// `rgba` is tightly packed top-down or bottom-up (OpenGL read is bottom-up).
    /// `flip_y` true when pixels come from `glReadPixels` (origin bottom-left).
    pub fn encode_rgba(&mut self, rgba: &[u8], flip_y: bool) -> Result<(Vec<u8>, bool), String> {
        let w = self.width as usize;
        let h = self.height as usize;
        let expected = w * h * 4;
        if rgba.len() < expected {
            return Err(format!(
                "rgba too short: {} < {} ({}x{})",
                rgba.len(),
                expected,
                w,
                h
            ));
        }

        let yuv = rgba_to_i420(rgba, w, h, flip_y);
        let force = self.frame_index % self.force_idr_every == 0;
        if force {
            // Request IDR by reconfiguring keyframe interval via frame type if API allows.
            // openh264 0.6: encode returns bitstream; first frames are IDR.
        }
        let bit_stream = self
            .enc
            .encode(&yuv)
            .map_err(|e| format!("encode: {e:?}"))?;

        let nals = bit_stream.to_vec();
        let mut is_idr = force || self.frame_index == 0;
        // Detect IDR NAL type 5 in Annex-B stream
        let mut i = 0;
        while i + 4 < nals.len() {
            if nals[i..].starts_with(&[0, 0, 0, 1]) {
                let nt = nals[i + 4] & 0x1f;
                if nt == 5 {
                    is_idr = true;
                    break;
                }
                i += 4;
            } else if nals[i..].starts_with(&[0, 0, 1]) {
                let nt = nals[i + 3] & 0x1f;
                if nt == 5 {
                    is_idr = true;
                    break;
                }
                i += 3;
            } else {
                i += 1;
            }
        }

        self.frame_index = self.frame_index.wrapping_add(1);
        if nals.is_empty() {
            return Err("encoder produced empty bitstream".into());
        }
        Ok((nals, is_idr))
    }

    pub fn pts_us(&self) -> u64 {
        self.origin.elapsed().as_micros() as u64
    }

    pub fn frame_index(&self) -> u64 {
        self.frame_index
    }
}

/// Push an encoded frame into the shared slot (and capture SPS/PPS on first IDR).
pub fn publish_stereo(
    slot: &LiveVideoSlot,
    left: Vec<u8>,
    right: Vec<u8>,
    is_idr: bool,
    pts_us: u64,
    frame_index: u64,
) {
    let mut g = match slot.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if is_idr && g.param_sets.is_empty() {
        g.param_sets = extract_param_sets(&left);
        if !g.param_sets.is_empty() {
            info!("live video SPS/PPS {}B", g.param_sets.len());
        }
    }
    g.left = left;
    g.right = right;
    g.is_idr = is_idr;
    g.pts_us = pts_us;
    g.frame_index = frame_index;
}

fn extract_param_sets(idr: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 3 < idr.len() {
        let sc = if idr[i..].starts_with(&[0, 0, 0, 1]) {
            4
        } else if idr[i..].starts_with(&[0, 0, 1]) {
            3
        } else {
            i += 1;
            continue;
        };
        let nal_start = i + sc;
        if nal_start >= idr.len() {
            break;
        }
        let nal_type = idr[nal_start] & 0x1f;
        let mut j = nal_start + 1;
        while j + 3 < idr.len() {
            if idr[j..].starts_with(&[0, 0, 0, 1]) || idr[j..].starts_with(&[0, 0, 1]) {
                break;
            }
            j += 1;
        }
        if nal_type == 7 || nal_type == 8 {
            out.extend_from_slice(&idr[i..j]);
        }
        if nal_type == 5 {
            break;
        }
        i = j;
    }
    out
}

/// BT.601 full-range-ish RGB→I420 (simple, good enough for a debug viz).
fn rgba_to_i420(rgba: &[u8], w: usize, h: usize, flip_y: bool) -> YuvOwned {
    let mut y = vec![0u8; w * h];
    let mut u = vec![0u8; (w / 2) * (h / 2)];
    let mut v = vec![0u8; (w / 2) * (h / 2)];

    for row in 0..h {
        let src_row = if flip_y { h - 1 - row } else { row };
        for col in 0..w {
            let i = (src_row * w + col) * 4;
            let r = rgba[i] as i32;
            let g = rgba[i + 1] as i32;
            let b = rgba[i + 2] as i32;
            // ITU-R BT.601
            let yy = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
            y[row * w + col] = yy.clamp(0, 255) as u8;
        }
    }
    for row in 0..(h / 2) {
        for col in 0..(w / 2) {
            let mut rs = 0i32;
            let mut gs = 0i32;
            let mut bs = 0i32;
            for dy in 0..2 {
                for dx in 0..2 {
                    let rr = if flip_y {
                        h - 1 - (row * 2 + dy)
                    } else {
                        row * 2 + dy
                    };
                    let cc = col * 2 + dx;
                    let i = (rr * w + cc) * 4;
                    rs += rgba[i] as i32;
                    gs += rgba[i + 1] as i32;
                    bs += rgba[i + 2] as i32;
                }
            }
            rs /= 4;
            gs /= 4;
            bs /= 4;
            let uu = ((-38 * rs - 74 * gs + 112 * bs + 128) >> 8) + 128;
            let vv = ((112 * rs - 94 * gs - 18 * bs + 128) >> 8) + 128;
            u[row * (w / 2) + col] = uu.clamp(0, 255) as u8;
            v[row * (w / 2) + col] = vv.clamp(0, 255) as u8;
        }
    }

    YuvOwned { y, u, v, w, h }
}

struct YuvOwned {
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
    w: usize,
    h: usize,
}

impl YUVSource for YuvOwned {
    fn dimensions(&self) -> (usize, usize) {
        (self.w, self.h)
    }
    fn strides(&self) -> (usize, usize, usize) {
        (self.w, self.w / 2, self.w / 2)
    }
    fn y(&self) -> &[u8] {
        &self.y
    }
    fn u(&self) -> &[u8] {
        &self.u
    }
    fn v(&self) -> &[u8] {
        &self.v
    }
}

