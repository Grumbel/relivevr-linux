//! H.264 encode of RGBA frames for the ReliveVR video channel.
//!
//! Backends (select with `RELIVEVR_ENCODER`):
//!   - `auto`     — try FFmpeg hardware (nvenc → vaapi → qsv), then libx264, then OpenH264
//!   - `ffmpeg`   — FFmpeg auto-pick among hw + libx264
//!   - `nvenc`    — NVIDIA NVENC via FFmpeg
//!   - `vaapi`    — VA-API (AMD / Intel) via FFmpeg
//!   - `qsv`      — Intel Quick Sync via FFmpeg
//!   - `x264`     — FFmpeg libx264 ultrafast / zerolatency
//!   - `openh264` — software OpenH264 (original path)
//!
//! Produces Annex-B NAL units suitable for `make_video_frame_packet`.

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use openh264::encoder::{Encoder, EncoderConfig};
use openh264::formats::YUVSource;
use openh264::OpenH264API;
use tracing::{info, warn};

/// Client StartRequest uses DisplayWidth/Height **1440×1440** at ≈74.8 Hz.
/// Default matches native resolution; hardware encode (nvenc/vaapi/qsv) is
/// expected for a usable framerate. OpenH264 at 1440²×75 is very heavy —
/// set `RELIVEVR_ENCODE_W/H=720` (or force `RELIVEVR_ENCODER=openh264` and
/// lower res) if you stay on software.
///
/// Override examples:
///   RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720
///   RELIVEVR_ENCODE_FPS=75
///   RELIVEVR_ENCODE_BITRATE=12000000
pub fn encode_width() -> u32 {
    std::env::var("RELIVEVR_ENCODE_W")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1440)
}

pub fn encode_height() -> u32 {
    std::env::var("RELIVEVR_ENCODE_H")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1440)
}

/// Headset reported rate (Daydream StartRequest ≈ 74.8).
pub fn target_encode_fps() -> f32 {
    std::env::var("RELIVEVR_ENCODE_FPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(75.0)
}

/// Target bitrate per eye (bps).
/// Default **10 Mbps/eye** (≈20 Mbps stereo) — workable for 1440² @ 75 Hz on
/// modern HW encoders. Client StartRequest advertises 50 Mbps; raise with
/// RELIVEVR_ENCODE_BITRATE if you want more quality / less blockiness.
pub fn target_bitrate_bps() -> u32 {
    std::env::var("RELIVEVR_ENCODE_BITRATE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000_000)
}

// Back-compat names used at call sites that capture size once at init.
pub fn encode_dims() -> (u32, u32) {
    let mut w = encode_width();
    let mut h = encode_height();
    // OpenH264 / most HW encoders prefer even dimensions
    if w % 2 == 1 {
        w += 1;
    }
    if h % 2 == 1 {
        h += 1;
    }
    (w, h)
}

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

// ---------------------------------------------------------------------------
// Backend selection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EncoderKind {
    Auto,
    Ffmpeg,
    Nvenc,
    Vaapi,
    Qsv,
    X264,
    OpenH264,
}

fn encoder_kind() -> EncoderKind {
    match std::env::var("RELIVEVR_ENCODER")
        .unwrap_or_else(|_| "auto".into())
        .to_lowercase()
        .as_str()
    {
        "openh264" | "soft" | "software" => EncoderKind::OpenH264,
        "ffmpeg" => EncoderKind::Ffmpeg,
        "nvenc" | "nvidia" => EncoderKind::Nvenc,
        "vaapi" | "amd" | "intel" => EncoderKind::Vaapi,
        "qsv" => EncoderKind::Qsv,
        "x264" | "libx264" => EncoderKind::X264,
        _ => EncoderKind::Auto,
    }
}

fn ffmpeg_bin() -> String {
    std::env::var("RELIVEVR_FFMPEG").unwrap_or_else(|_| "ffmpeg".into())
}

// ---------------------------------------------------------------------------
// Public encoder (same API as before)
// ---------------------------------------------------------------------------

enum Backend {
    Soft(SoftEncoder),
    Ffmpeg(FfmpegEncoder),
}

/// H.264 encoder; call `encode_rgba` from the GL thread after readback.
pub struct H264Encoder {
    backend: Backend,
    width: u32,
    height: u32,
    frame_index: u64,
    origin: Instant,
    force_idr_every: u64,
}

impl H264Encoder {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let kind = encoder_kind();
        let bitrate = target_bitrate_bps();
        let fps = target_encode_fps();
        let gop = fps.max(1.0).round() as u32; // ~1 s IDR interval

        let backend = match kind {
            EncoderKind::OpenH264 => {
                info!("encoder backend: OpenH264 (forced)");
                Backend::Soft(SoftEncoder::new(width, height, bitrate, fps)?)
            }
            EncoderKind::Nvenc => {
                info!("encoder backend: FFmpeg h264_nvenc (forced)");
                Backend::Ffmpeg(FfmpegEncoder::spawn(
                    width, height, fps, bitrate, gop, FfmpegCodec::Nvenc,
                )?)
            }
            EncoderKind::Vaapi => {
                info!("encoder backend: FFmpeg h264_vaapi (forced)");
                Backend::Ffmpeg(FfmpegEncoder::spawn(
                    width, height, fps, bitrate, gop, FfmpegCodec::Vaapi,
                )?)
            }
            EncoderKind::Qsv => {
                info!("encoder backend: FFmpeg h264_qsv (forced)");
                Backend::Ffmpeg(FfmpegEncoder::spawn(
                    width, height, fps, bitrate, gop, FfmpegCodec::Qsv,
                )?)
            }
            EncoderKind::X264 => {
                info!("encoder backend: FFmpeg libx264 (forced)");
                Backend::Ffmpeg(FfmpegEncoder::spawn(
                    width, height, fps, bitrate, gop, FfmpegCodec::X264,
                )?)
            }
            EncoderKind::Ffmpeg | EncoderKind::Auto => {
                // Prefer hardware, then libx264, then OpenH264.
                let order = [
                    FfmpegCodec::Nvenc,
                    FfmpegCodec::Vaapi,
                    FfmpegCodec::Qsv,
                    FfmpegCodec::X264,
                ];
                let mut last_err = String::new();
                let mut chosen = None;
                for c in order {
                    match FfmpegEncoder::spawn(width, height, fps, bitrate, gop, c) {
                        Ok(enc) => {
                            info!("encoder backend: FFmpeg {} (auto)", c.name());
                            chosen = Some(enc);
                            break;
                        }
                        Err(e) => {
                            warn!("FFmpeg {} unavailable: {e}", c.name());
                            last_err = e;
                        }
                    }
                }
                if let Some(enc) = chosen {
                    Backend::Ffmpeg(enc)
                } else if kind == EncoderKind::Auto {
                    info!("encoder backend: OpenH264 (FFmpeg unavailable: {last_err})");
                    Backend::Soft(SoftEncoder::new(width, height, bitrate, fps)?)
                } else {
                    return Err(format!(
                        "FFmpeg encoder failed and RELIVEVR_ENCODER=ffmpeg (no OpenH264 fallback): {last_err}"
                    ));
                }
            }
        };

        Ok(Self {
            backend,
            width,
            height,
            frame_index: 0,
            origin: Instant::now(),
            force_idr_every: gop.max(1) as u64,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }

    /// `rgba` is tightly packed. `flip_y` true when pixels come from `glReadPixels`
    /// (origin bottom-left).
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

        let force_idr = self.frame_index % self.force_idr_every == 0 || self.frame_index == 0;

        let (nals, is_idr) = match &mut self.backend {
            Backend::Soft(enc) => enc.encode_rgba(rgba, w, h, flip_y, force_idr)?,
            Backend::Ffmpeg(enc) => enc.encode_rgba(rgba, w, h, flip_y, force_idr)?,
        };

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

impl Drop for H264Encoder {
    fn drop(&mut self) {
        if let Backend::Ffmpeg(enc) = &mut self.backend {
            enc.shutdown();
        }
    }
}

// ---------------------------------------------------------------------------
// OpenH264 software backend
// ---------------------------------------------------------------------------

struct SoftEncoder {
    enc: Encoder,
}

impl SoftEncoder {
    fn new(width: u32, height: u32, bitrate: u32, fps: f32) -> Result<Self, String> {
        let cfg = EncoderConfig::new()
            .set_bitrate_bps(bitrate)
            .max_frame_rate(fps)
            .enable_skip_frame(false);
        let api = OpenH264API::from_source();
        let enc = Encoder::with_api_config(api, cfg)
            .map_err(|e| format!("OpenH264 init: {e:?}"))?;
        if width * height > 720 * 720 {
            warn!(
                "OpenH264 at {width}x{height} @ {fps:.0} fps is CPU-heavy;                  prefer RELIVEVR_ENCODER=nvenc|vaapi or lower RELIVEVR_ENCODE_W/H"
            );
        }
        info!("OpenH264 encoder ready {width}x{height} bitrate={bitrate} fps={fps:.0}");
        Ok(Self { enc })
    }

    fn encode_rgba(
        &mut self,
        rgba: &[u8],
        w: usize,
        h: usize,
        flip_y: bool,
        _force_idr: bool,
    ) -> Result<(Vec<u8>, bool), String> {
        let yuv = rgba_to_i420(rgba, w, h, flip_y);
        let bit_stream = self
            .enc
            .encode(&yuv)
            .map_err(|e| format!("encode: {e:?}"))?;
        let nals = bit_stream.to_vec();
        let is_idr = annexb_has_idr(&nals);
        Ok((nals, is_idr))
    }
}

// ---------------------------------------------------------------------------
// FFmpeg pipe backend (hardware or libx264)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum FfmpegCodec {
    Nvenc,
    Vaapi,
    Qsv,
    X264,
}

impl FfmpegCodec {
    fn name(self) -> &'static str {
        match self {
            Self::Nvenc => "h264_nvenc",
            Self::Vaapi => "h264_vaapi",
            Self::Qsv => "h264_qsv",
            Self::X264 => "libx264",
        }
    }
}

struct FfmpegEncoder {
    child: Child,
    /// Pre-allocated NV12 staging buffer.
    nv12: Vec<u8>,
    width: u32,
    height: u32,
    codec: FfmpegCodec,
}

impl FfmpegEncoder {
    fn spawn(
        width: u32,
        height: u32,
        fps: f32,
        bitrate: u32,
        gop: u32,
        codec: FfmpegCodec,
    ) -> Result<Self, String> {
        let bin = ffmpeg_bin();
        let fps_s = format!("{fps:.3}");
        let size = format!("{width}x{height}");
        let br = format!("{bitrate}");
        let gop_s = gop.to_string();
        let maxrate = br.clone();
        let bufsize = (bitrate / 2).to_string();

        let mut args: Vec<String> = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-nostdin".into(),
            "-fflags".into(),
            "+nobuffer".into(),
            "-flags".into(),
            "low_delay".into(),
        ];

        // VA-API needs the device flag before the input.
        if matches!(codec, FfmpegCodec::Vaapi) {
            let dev = std::env::var("RELIVEVR_VAAPI_DEVICE")
                .unwrap_or_else(|_| "/dev/dri/renderD128".into());
            args.push("-vaapi_device".into());
            args.push(dev);
        }

        // Raw NV12 input on stdin.
        args.extend([
            "-f".into(),
            "rawvideo".into(),
            "-pix_fmt".into(),
            "nv12".into(),
            "-s".into(),
            size,
            "-framerate".into(),
            fps_s,
            "-i".into(),
            "pipe:0".into(),
        ]);

        match codec {
            FfmpegCodec::Vaapi => {
                // Upload to GPU then encode.
                args.extend([
                    "-vf".into(),
                    "format=nv12,hwupload".into(),
                    "-c:v".into(),
                    "h264_vaapi".into(),
                    "-bf".into(),
                    "0".into(),
                    "-g".into(),
                    gop_s,
                    "-b:v".into(),
                    br,
                ]);
            }
            FfmpegCodec::Nvenc => {
                args.extend([
                    "-c:v".into(),
                    "h264_nvenc".into(),
                    "-preset".into(),
                    "p1".into(), // fastest
                    "-tune".into(),
                    "ll".into(), // low latency
                    "-profile:v".into(),
                    "baseline".into(),
                    "-bf".into(),
                    "0".into(),
                    "-g".into(),
                    gop_s,
                    "-b:v".into(),
                    br,
                    "-maxrate".into(),
                    maxrate,
                    "-bufsize".into(),
                    bufsize,
                    "-rc".into(),
                    "cbr".into(),
                ]);
            }
            FfmpegCodec::Qsv => {
                args.extend([
                    "-c:v".into(),
                    "h264_qsv".into(),
                    "-bf".into(),
                    "0".into(),
                    "-g".into(),
                    gop_s,
                    "-b:v".into(),
                    br,
                    "-look_ahead".into(),
                    "0".into(),
                ]);
            }
            FfmpegCodec::X264 => {
                args.extend([
                    "-c:v".into(),
                    "libx264".into(),
                    "-preset".into(),
                    "ultrafast".into(),
                    "-tune".into(),
                    "zerolatency".into(),
                    "-profile:v".into(),
                    "baseline".into(),
                    "-bf".into(),
                    "0".into(),
                    "-g".into(),
                    gop_s,
                    "-b:v".into(),
                    br,
                    "-maxrate".into(),
                    maxrate,
                    "-bufsize".into(),
                    bufsize,
                ]);
            }
        }

        args.extend([
            "-an".into(),
            "-flush_packets".into(),
            "1".into(),
            "-f".into(),
            "h264".into(),
            "pipe:1".into(),
        ]);

        let mut child = Command::new(&bin)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn {bin}: {e}"))?;

        // Quick health-check: process must still be alive after a short moment.
        // (Some hw encoders fail immediately if the device is missing.)
        std::thread::sleep(std::time::Duration::from_millis(50));
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut err = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    let _ = stderr.read_to_string(&mut err);
                }
                return Err(format!(
                    "{bin} {} exited immediately ({status}): {err}",
                    codec.name()
                ));
            }
            Ok(None) => {}
            Err(e) => return Err(format!("wait {bin}: {e}")),
        }

        let nv12_len = (width as usize) * (height as usize) * 3 / 2;
        info!(
            "FFmpeg {} ready {}x{} bitrate={bitrate} fps={fps:.0} gop={gop}",
            codec.name(),
            width,
            height
        );
        Ok(Self {
            child,
            nv12: vec![0u8; nv12_len],
            width,
            height,
            codec,
        })
    }

    fn encode_rgba(
        &mut self,
        rgba: &[u8],
        w: usize,
        h: usize,
        flip_y: bool,
        _force_idr: bool,
    ) -> Result<(Vec<u8>, bool), String> {
        rgba_to_nv12(rgba, w, h, flip_y, &mut self.nv12);

        {
            let stdin = self
                .child
                .stdin
                .as_mut()
                .ok_or("ffmpeg stdin closed")?;
            stdin
                .write_all(&self.nv12)
                .map_err(|e| format!("ffmpeg stdin write: {e}"))?;
            stdin.flush().map_err(|e| format!("ffmpeg stdin flush: {e}"))?;
        }

        // Read one access unit (Annex-B). FFmpeg with -f h264 emits start-code
        // delimited NALs; we accumulate until we have a complete AU that ends
        // at the next start code of a non-VCL or we hit a short read timeout.
        let nals = self.read_access_unit()?;
        let is_idr = annexb_has_idr(&nals);
        Ok((nals, is_idr))
    }

    /// Blocking read of one H.264 access unit from ffmpeg stdout.
    ///
    /// Strategy: read chunks until we see a start code that begins a new VCL
    /// NAL *after* we already have at least one VCL NAL, or until a short idle.
    fn read_access_unit(&mut self) -> Result<Vec<u8>, String> {
        let stdout = self
            .child
            .stdout
            .as_mut()
            .ok_or("ffmpeg stdout closed")?;

        let mut buf = Vec::with_capacity(64 * 1024);
        let mut tmp = [0u8; 8192];
        let deadline = Instant::now() + std::time::Duration::from_millis(500);

        // Set non-blocking would be ideal; for simplicity we use a read loop
        // with a wall-clock deadline and rely on zerolatency producing output
        // promptly after each written frame.
        loop {
            if Instant::now() > deadline {
                if buf.is_empty() {
                    // Check if process died
                    if let Ok(Some(st)) = self.child.try_wait() {
                        let mut err = String::new();
                        if let Some(mut stderr) = self.child.stderr.take() {
                            let _ = stderr.read_to_string(&mut err);
                        }
                        return Err(format!(
                            "ffmpeg {} died ({st}) while waiting for AU: {err}",
                            self.codec.name()
                        ));
                    }
                    return Err("ffmpeg AU read timeout (no data)".into());
                }
                // Return whatever we have — likely a complete AU.
                break;
            }

            match stdout.read(&mut tmp) {
                Ok(0) => {
                    if buf.is_empty() {
                        return Err("ffmpeg stdout EOF".into());
                    }
                    break;
                }
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                    // Heuristic: if we have SPS/PPS/IDR or a P-slice and the
                    // last bytes look like the end of a NAL (next would be a
                    // new start code), and we have not seen more data for a
                    // brief moment, stop. For zerolatency, one write → one AU.
                    if buf.len() > 32 && annexb_looks_complete(&buf) {
                        // Peek whether more data is immediately available by
                        // doing one non-blocking-ish extra read with a short
                        // deadline — if nothing comes, we're done.
                        // (We cannot easily set O_NONBLOCK portably without
                        // extra crates; instead break after first complete AU
                        // signal.)
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(format!("ffmpeg stdout read: {e}")),
            }
        }

        if buf.is_empty() {
            return Err("ffmpeg produced empty AU".into());
        }
        Ok(buf)
    }

    fn shutdown(&mut self) {
        let _ = self.child.stdin.take(); // close stdin → ffmpeg drains
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for FfmpegEncoder {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn annexb_has_idr(nals: &[u8]) -> bool {
    let mut i = 0;
    while i + 4 < nals.len() {
        if nals[i..].starts_with(&[0, 0, 0, 1]) {
            if (nals[i + 4] & 0x1f) == 5 {
                return true;
            }
            i += 4;
        } else if nals[i..].starts_with(&[0, 0, 1]) {
            if (nals[i + 3] & 0x1f) == 5 {
                return true;
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    false
}

/// True if the buffer contains at least one VCL NAL (type 1 or 5).
fn annexb_looks_complete(nals: &[u8]) -> bool {
    let mut i = 0;
    while i + 4 < nals.len() {
        let (sc, nt) = if nals[i..].starts_with(&[0, 0, 0, 1]) {
            (4, nals[i + 4] & 0x1f)
        } else if nals[i..].starts_with(&[0, 0, 1]) {
            (3, nals[i + 3] & 0x1f)
        } else {
            i += 1;
            continue;
        };
        if nt == 1 || nt == 5 {
            return true;
        }
        i += sc;
    }
    false
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

/// BT.601 RGB→I420 for OpenH264.
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

/// BT.601 RGB→NV12 into a pre-sized buffer (Y plane then interleaved UV).
fn rgba_to_nv12(rgba: &[u8], w: usize, h: usize, flip_y: bool, out: &mut [u8]) {
    let y_size = w * h;
    debug_assert!(out.len() >= y_size + y_size / 2);

    for row in 0..h {
        let src_row = if flip_y { h - 1 - row } else { row };
        for col in 0..w {
            let i = (src_row * w + col) * 4;
            let r = rgba[i] as i32;
            let g = rgba[i + 1] as i32;
            let b = rgba[i + 2] as i32;
            let yy = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
            out[row * w + col] = yy.clamp(0, 255) as u8;
        }
    }

    let uv = &mut out[y_size..];
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
            let o = row * w + col * 2;
            uv[o] = uu.clamp(0, 255) as u8;
            uv[o + 1] = vv.clamp(0, 255) as u8;
        }
    }
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
