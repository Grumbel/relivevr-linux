//! H.264 / HEVC encode of NV12 frames for the ReliveVR video channel.
//!
//! Backends (select with `--encoder` / `RELIVEVR_ENCODER`):
//!   - `auto`     — h264_nvenc → h264_vaapi → h264_qsv → libx264 → OpenH264
//!   - `ffmpeg`   — same as auto, without the OpenH264 fallback
//!   - `hevc` / `x265` — hevc_nvenc → hevc_vaapi → libx265 (only useful if the
//!     client's StartRequest asks for `hevc`; the Daydream client asks for `avc`)
//!   - `nvenc` / `vaapi` / `qsv` — H.264 on that hardware API
//!   - `hevc_vaapi` / `hevc_nvenc` — HEVC on that hardware API
//!   - `x264`     — FFmpeg libx264 ultrafast / zerolatency
//!   - `openh264` — software OpenH264
//!
//! Every backend is strictly one-in / one-out: each submitted frame yields
//! exactly one access unit, in order. FFmpeg backends emit an AUD at the start
//! of every AU, which is how the stdout reader frames them; the AUD is stripped
//! before the AU is handed out. Periodic IDRs (with in-band parameter sets)
//! come every `gop` frames — the UDP sender starts a stream only on an IDR.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use openh264::encoder::{Encoder as OpenH264Encoder, EncoderConfig};
use openh264::formats::YUVSource;
use openh264::OpenH264API;
use tracing::{info, warn};

use crate::config::{self, PtsSensor};
pub use crate::config::SimMode;

/// Client StartRequest uses DisplayWidth/Height **1440×1440**.
pub fn encode_width() -> u32 {
    config::get().width
}

pub fn encode_height() -> u32 {
    config::get().height
}

/// Render/encode rate. Daydream / Mirage Solo reports ≈74.9 Hz via UpdateRequest.
pub fn target_encode_fps() -> f32 {
    config::get().fps
}

/// Low-fps simulation (`--sim-fps`): the scene is rendered, and the HMD pose
/// sampled, at this rate only, like a game that can't keep up with the
/// display. `None` renders every encode tick.
pub fn sim_fps() -> Option<f32> {
    config::get().sim_fps.filter(|&f| f > 0.0)
}

pub fn sim_mode() -> SimMode {
    config::get().sim_mode
}

/// `--pts-sensor latest` stamps VideoData `ptsSensor` with the newest
/// HMD pose at send time (old tip 087.1) instead of the pose the frame was
/// rendered with (default, as the ALVR fork). With `latest` the client's
/// reprojection corrects from the wrong pose by the render→send age.
pub fn pts_sensor_latest() -> bool {
    config::get().pts_sensor == PtsSensor::Latest
}

/// Per-eye field of view (degrees, symmetric, horizontal = vertical) that the
/// eyes are rendered with. It is advertised in HelloResponse
/// `HorizontalFOV`/`VerticalFOV`; the client passes it to
/// `gvr_buffer_viewport_set_source_fov` (±fov/2 on every side) and GVR does
/// the lens distortion — so rendering and advertised FOV must match exactly.
pub fn stream_fov_deg() -> f32 {
    config::get().fov
}

/// Target bitrate per eye (bps). Default 10 Mbps/eye (≈20 Mbps stereo).
pub fn target_bitrate_bps() -> u32 {
    config::get().bitrate
}

/// IDR interval in frames (`--gop`, default ≈1 s). The sender
/// waits for an IDR when a stream (re)starts, so this bounds startup delay.
pub fn target_gop() -> u32 {
    config::get()
        .gop
        .unwrap_or_else(|| target_encode_fps().round().max(1.0) as u32)
}

pub fn encode_dims() -> (u32, u32) {
    let mut w = encode_width();
    let mut h = encode_height();
    // NV12 needs even dimensions
    if w % 2 == 1 {
        w += 1;
    }
    if h % 2 == 1 {
        h += 1;
    }
    (w, h)
}

// ---------------------------------------------------------------------------
// Shared slot between the encode threads and the UDP sender
// ---------------------------------------------------------------------------

/// One encoded stereo pair.
#[derive(Clone)]
pub struct StereoFrame {
    /// Annex-B NALs — left eye (frmType 0).
    pub left: Vec<u8>,
    /// Annex-B NALs — right eye (frmType 1).
    pub right: Vec<u8>,
    /// Both eyes are IDR (the GOPs of both encoders are aligned).
    pub is_idr: bool,
    pub frame_index: u64,
    /// Client `/hmd/pose` sample `time` used to render this pair (ptsSensor).
    pub pose_time: u64,
    /// When the pair was rendered (for ptsEncoderLat / ptsServerLat).
    pub rendered_at: Instant,
    /// When the pair came out of the encoders.
    pub encoded_at: Instant,
}

/// Max queued pairs before the sender is considered stalled. On overflow the
/// queue is flushed and the sender resyncs on the next IDR (dropping a P-frame
/// would corrupt every following frame until then).
const MAX_QUEUED_FRAMES: usize = 6;

#[derive(Default)]
pub struct LiveVideo {
    /// Encoded pairs in order; the sender pops from the front.
    pub queue: VecDeque<StereoFrame>,
    /// Set when frames were discarded; sender must resync on an IDR.
    pub overflowed: bool,
    /// SPS+PPS (AVC) or VPS+SPS+PPS (HEVC) for the VideoInit trailer.
    pub param_sets: Vec<u8>,
    /// "avc" or "hevc" — VideoInit CodecID (Windows native is hevc).
    pub codec_id: String,
    /// Wakes the UDP sender when a frame is queued.
    pub notify: Arc<tokio::sync::Notify>,
}

pub type LiveVideoSlot = Arc<Mutex<LiveVideo>>;

pub fn new_slot() -> LiveVideoSlot {
    Arc::new(Mutex::new(LiveVideo::default()))
}

/// Publish SPS/PPS (from encoder warm-up) before any live frame.
pub fn publish_param_sets(slot: &LiveVideoSlot, params: Vec<u8>, codec_id: &str) {
    if params.is_empty() {
        return;
    }
    let Ok(mut g) = slot.lock() else { return };
    if g.param_sets.is_empty() {
        g.param_sets = params;
        g.codec_id = codec_id.to_string();
        info!("live video {} param sets {}B", g.codec_id, g.param_sets.len());
    }
}

pub fn publish_stereo(slot: &LiveVideoSlot, frame: StereoFrame) {
    let Ok(mut g) = slot.lock() else { return };
    if g.queue.len() >= MAX_QUEUED_FRAMES {
        g.queue.clear();
        g.overflowed = true;
    }
    g.queue.push_back(frame);
    g.notify.notify_one();
}

// ---------------------------------------------------------------------------
// Backend selection
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum FfmpegCodec {
    Nvenc,
    Vaapi,
    Qsv,
    X264,
    HevcNvenc,
    HevcVaapi,
    X265,
}

impl FfmpegCodec {
    fn name(self) -> &'static str {
        match self {
            Self::Nvenc => "h264_nvenc",
            Self::Vaapi => "h264_vaapi",
            Self::Qsv => "h264_qsv",
            Self::X264 => "libx264",
            Self::HevcNvenc => "hevc_nvenc",
            Self::HevcVaapi => "hevc_vaapi",
            Self::X265 => "libx265",
        }
    }

    fn is_hevc(self) -> bool {
        matches!(self, Self::HevcNvenc | Self::HevcVaapi | Self::X265)
    }

    fn is_vaapi(self) -> bool {
        matches!(self, Self::Vaapi | Self::HevcVaapi)
    }
}

#[derive(Clone, Copy, Debug)]
enum Choice {
    Ffmpeg(FfmpegCodec),
    OpenH264,
}

fn backend_order() -> Vec<Choice> {
    use Choice::*;
    use FfmpegCodec::*;
    // H.264 first: the Daydream client creates its MediaCodec decoder from its
    // own StartRequest `VideoCodec` ("avc") and ignores VideoInit CodecID, so
    // HEVC is fed into an AVC decoder (solid green).
    let auto = vec![
        Ffmpeg(Nvenc),
        Ffmpeg(Vaapi),
        Ffmpeg(Qsv),
        Ffmpeg(X264),
    ];
    match config::get().encoder.to_lowercase().as_str()
    {
        "openh264" | "soft" | "software" => vec![OpenH264],
        "ffmpeg" => auto,
        "nvenc" | "nvidia" | "h264_nvenc" => vec![Ffmpeg(Nvenc)],
        "vaapi" | "amd" | "intel" | "h264_vaapi" => vec![Ffmpeg(Vaapi)],
        "qsv" | "h264_qsv" => vec![Ffmpeg(Qsv)],
        "x264" | "libx264" => vec![Ffmpeg(X264)],
        "hevc_vaapi" => vec![Ffmpeg(HevcVaapi)],
        "hevc_nvenc" => vec![Ffmpeg(HevcNvenc)],
        "hevc" | "h265" | "x265" | "libx265" => {
            vec![Ffmpeg(HevcNvenc), Ffmpeg(HevcVaapi), Ffmpeg(X265)]
        }
        _ => {
            let mut v = auto;
            v.push(OpenH264);
            v
        }
    }
}

fn ffmpeg_bin() -> String {
    config::get().ffmpeg.clone()
}

// ---------------------------------------------------------------------------
// Public encoder
// ---------------------------------------------------------------------------

/// Encoder split into an input half (submit NV12) and an output half (receive
/// AUs), so submission and collection can run on different threads.
pub struct VideoEncoder {
    pub input: EncoderInput,
    pub output: EncoderOutput,
    pub codec_id: &'static str,
    /// Parameter sets from the warm-up IDR.
    pub param_sets: Vec<u8>,
}

pub struct EncoderInput {
    backend: InputBackend,
    frame_len: usize,
}

enum InputBackend {
    Soft {
        enc: OpenH264Encoder,
        tx: Sender<Vec<u8>>,
        width: usize,
        height: usize,
        gop: u64,
        frames: u64,
    },
    Ffmpeg {
        child: Child,
        stdin: Option<ChildStdin>,
        stderr: Arc<Mutex<String>>,
    },
}

pub struct EncoderOutput {
    rx: Receiver<Vec<u8>>,
    stderr: Option<Arc<Mutex<String>>>,
}

impl VideoEncoder {
    /// Open the first working backend from `--encoder` and warm it up
    /// with one black frame (consumes the first IDR, yields parameter sets).
    pub fn open(width: u32, height: u32) -> Result<Self, String> {
        let bitrate = target_bitrate_bps();
        let fps = target_encode_fps();
        let gop = target_gop();
        let mut last_err = String::from("no encoder backend configured");
        for choice in backend_order() {
            let res = match choice {
                Choice::Ffmpeg(c) => open_ffmpeg(width, height, fps, bitrate, gop, c),
                Choice::OpenH264 => open_openh264(width, height, fps, bitrate, gop),
            };
            let mut enc = match res {
                Ok(enc) => enc,
                Err(e) => {
                    warn!("encoder {choice:?} unavailable: {e}");
                    last_err = e;
                    continue;
                }
            };
            match enc.warm_up(width, height) {
                Ok(()) => {
                    info!(
                        "encoder backend {choice:?} ready {width}x{height} bitrate={bitrate} fps={fps:.0} gop={gop} codec={} params={}B",
                        enc.codec_id,
                        enc.param_sets.len()
                    );
                    return Ok(enc);
                }
                Err(e) => {
                    warn!("encoder {choice:?} warm-up failed: {e}");
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    fn warm_up(&mut self, width: u32, height: u32) -> Result<(), String> {
        let black = black_nv12(width as usize, height as usize);
        self.input.submit(&black)?;
        // ffmpeg keeps running (waiting for stdin EOF) when the encoder fails to
        // open, so watch stderr instead of waiting for the full timeout.
        let deadline = Instant::now() + Duration::from_secs(5);
        let au = loop {
            match self.output.recv(Duration::from_millis(50)) {
                Ok(au) => break au,
                Err(e) if Instant::now() >= deadline => return Err(e),
                Err(e) => {
                    let err = self.output.stderr.as_ref().map(snapshot).unwrap_or_default();
                    if err.contains("Could not open encoder")
                        || err.contains("Error while opening encoder")
                        || err.contains("Error initializing")
                        || e.contains("output closed")
                    {
                        return Err(e);
                    }
                }
            }
        };
        self.param_sets = extract_param_sets(&au, self.codec_id == "hevc");
        if self.param_sets.is_empty() {
            return Err(format!("warm-up AU ({}B) has no parameter sets", au.len()));
        }
        Ok(())
    }
}

impl EncoderInput {
    /// Submit one tightly packed NV12 frame (Y plane, then interleaved UV).
    pub fn submit(&mut self, nv12: &[u8]) -> Result<(), String> {
        if nv12.len() != self.frame_len {
            return Err(format!("NV12 frame is {}B, expected {}B", nv12.len(), self.frame_len));
        }
        match &mut self.backend {
            InputBackend::Soft { enc, tx, width, height, gop, frames } => {
                if *frames % *gop == 0 {
                    enc.force_intra_frame();
                }
                *frames += 1;
                let yuv = nv12_to_i420(nv12, *width, *height);
                let au = enc
                    .encode(&yuv)
                    .map_err(|e| format!("OpenH264 encode: {e:?}"))?
                    .to_vec();
                tx.send(au).map_err(|_| "OpenH264 output closed".to_string())
            }
            InputBackend::Ffmpeg { stdin, stderr, .. } => {
                let Some(stdin) = stdin.as_mut() else {
                    return Err("ffmpeg stdin closed".into());
                };
                stdin
                    .write_all(nv12)
                    .and_then(|_| stdin.flush())
                    .map_err(|e| format!("ffmpeg stdin: {e}; stderr: {}", snapshot(stderr)))
            }
        }
    }
}

impl Drop for EncoderInput {
    fn drop(&mut self) {
        if let InputBackend::Ffmpeg { child, stdin, .. } = &mut self.backend {
            drop(stdin.take());
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl EncoderOutput {
    /// Next access unit, in submission order.
    pub fn recv(&self, timeout: Duration) -> Result<Vec<u8>, String> {
        let stderr = || self.stderr.as_ref().map(snapshot).unwrap_or_default();
        match self.rx.recv_timeout(timeout) {
            Ok(au) => Ok(au),
            Err(RecvTimeoutError::Timeout) => Err(format!("AU timeout; stderr: {}", stderr())),
            Err(RecvTimeoutError::Disconnected) => {
                Err(format!("encoder output closed; stderr: {}", stderr()))
            }
        }
    }
}

fn snapshot(buf: &Arc<Mutex<String>>) -> String {
    buf.lock().map(|g| g.trim().to_string()).unwrap_or_default()
}

fn black_nv12(w: usize, h: usize) -> Vec<u8> {
    let mut v = vec![16u8; w * h * 3 / 2];
    for b in &mut v[w * h..] {
        *b = 128;
    }
    v
}

// ---------------------------------------------------------------------------
// OpenH264 software backend
// ---------------------------------------------------------------------------

fn open_openh264(width: u32, height: u32, fps: f32, bitrate: u32, gop: u32) -> Result<VideoEncoder, String> {
    let cfg = EncoderConfig::new()
        .set_bitrate_bps(bitrate)
        .max_frame_rate(fps)
        .enable_skip_frame(false);
    let enc = OpenH264Encoder::with_api_config(OpenH264API::from_source(), cfg)
        .map_err(|e| format!("OpenH264 init: {e:?}"))?;
    if width * height > 720 * 720 {
        warn!("OpenH264 at {width}x{height} @ {fps:.0} fps is CPU-heavy; prefer hardware or lower --width/--height");
    }
    let (tx, rx) = std::sync::mpsc::channel();
    Ok(VideoEncoder {
        input: EncoderInput {
            backend: InputBackend::Soft {
                enc,
                tx,
                width: width as usize,
                height: height as usize,
                gop: gop.max(1) as u64,
                frames: 0,
            },
            frame_len: (width * height * 3 / 2) as usize,
        },
        output: EncoderOutput { rx, stderr: None },
        codec_id: "avc",
        param_sets: Vec::new(),
    })
}

fn nv12_to_i420(nv12: &[u8], w: usize, h: usize) -> YuvOwned {
    let y = nv12[..w * h].to_vec();
    let uv = &nv12[w * h..];
    let n = (w / 2) * (h / 2);
    let mut u = Vec::with_capacity(n);
    let mut v = Vec::with_capacity(n);
    for pair in uv.chunks_exact(2) {
        u.push(pair[0]);
        v.push(pair[1]);
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

// ---------------------------------------------------------------------------
// FFmpeg pipe backend (hardware or libx264/libx265)
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn set_pipe_size(fd: i32, bytes: i32) {
    // Best-effort: a pipe larger than one encoded AU lets ffmpeg's single
    // write() (-avioflags direct) land in one piece.
    unsafe {
        let _ = libc::fcntl(fd, libc::F_SETPIPE_SZ, bytes);
    }
}

#[cfg(not(target_os = "linux"))]
fn set_pipe_size(_fd: i32, _bytes: i32) {}

fn ffmpeg_args(width: u32, height: u32, fps: f32, bitrate: u32, gop: u32, codec: FfmpegCodec) -> Vec<String> {
    let br = bitrate.to_string();
    let bufsize = (bitrate / 2).max(1).to_string();
    let gop_s = gop.to_string();
    let mut args: Vec<String> = ["-hide_banner", "-nostdin", "-loglevel", "warning"]
        .map(String::from)
        .to_vec();
    if codec.is_vaapi() {
        let dev = config::get().vaapi_device.clone();
        args.extend(["-vaapi_device".into(), dev]);
    }
    args.extend([
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        "nv12".into(),
        "-s:v".into(),
        format!("{width}x{height}"),
        "-r".into(),
        format!("{fps:.3}"),
        "-i".into(),
        "pipe:0".into(),
    ]);
    let s = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    match codec {
        FfmpegCodec::Vaapi | FfmpegCodec::HevcVaapi => {
            // async_depth 1: output AU N right after input N (default 2 delays one frame).
            args.extend(s(&["-vf", "format=nv12,hwupload", "-c:v", codec.name(), "-async_depth", "1"]));
            args.extend(s(&["-aud", "1", "-bf", "0", "-g", &gop_s, "-b:v", &br, "-maxrate", &br]));
        }
        FfmpegCodec::Nvenc | FfmpegCodec::HevcNvenc => {
            args.extend(s(&["-c:v", codec.name(), "-preset", "p1", "-tune", "ull", "-zerolatency", "1"]));
            args.extend(s(&["-delay", "0", "-aud", "1", "-forced-idr", "1", "-bf", "0", "-g", &gop_s]));
            args.extend(s(&["-rc", "cbr", "-b:v", &br, "-maxrate", &br, "-bufsize", &bufsize]));
        }
        FfmpegCodec::Qsv => {
            args.extend(s(&["-c:v", "h264_qsv", "-async_depth", "1", "-look_ahead", "0", "-aud", "1"]));
            args.extend(s(&["-idr_interval", "0", "-bf", "0", "-g", &gop_s, "-b:v", &br]));
        }
        FfmpegCodec::X264 => {
            // Do NOT pass sliced-threads=0 (blocks open-pipe AUs).
            args.extend(s(&["-c:v", "libx264", "-preset", "ultrafast", "-tune", "zerolatency"]));
            args.extend(s(&["-profile:v", "baseline", "-bf", "0", "-g", &gop_s, "-b:v", &br]));
            args.extend(s(&["-maxrate", &br, "-bufsize", &bufsize, "-x264-params", "repeat-headers=1:aud=1"]));
        }
        FfmpegCodec::X265 => {
            let x265 = format!(
                "repeat-headers=1:aud=1:keyint={gop}:min-keyint={gop}:bframes=0:rc-lookahead=0:scenecut=0:open-gop=0"
            );
            args.extend(s(&["-c:v", "libx265", "-preset", "ultrafast", "-tune", "zerolatency"]));
            args.extend(s(&["-b:v", &br, "-maxrate", &br, "-bufsize", &bufsize, "-x265-params", &x265]));
        }
    }
    // -avioflags direct: every AU is written with a single write() (no 32 KiB
    // AVIO chunking), so the reader rarely sees a partial AU.
    args.extend(s(&["-an", "-fps_mode", "passthrough", "-flush_packets", "1", "-avioflags", "direct"]));
    args.extend(s(&["-f", if codec.is_hevc() { "hevc" } else { "h264" }, "pipe:1"]));
    args
}

fn open_ffmpeg(
    width: u32,
    height: u32,
    fps: f32,
    bitrate: u32,
    gop: u32,
    codec: FfmpegCodec,
) -> Result<VideoEncoder, String> {
    let bin = ffmpeg_bin();
    let args = ffmpeg_args(width, height, fps, bitrate, gop, codec);
    tracing::debug!("ffmpeg cmdline: {} {}", bin, args.join(" "));

    // ffmpeg is RPATH-linked; an inherited LD_LIBRARY_PATH (nix shells) can
    // pull in libraries built against a different glibc and kill it at start.
    let mut child = Command::new(&bin)
        .args(&args)
        .env_remove("LD_LIBRARY_PATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn {bin}: {e}"))?;

    let stderr_buf = Arc::new(Mutex::new(String::new()));
    if let Some(mut stderr) = child.stderr.take() {
        let buf = Arc::clone(&stderr_buf);
        std::thread::Builder::new()
            .name(format!("ffmpeg-err-{}", codec.name()))
            .spawn(move || {
                let mut tmp = [0u8; 4096];
                while let Ok(n) = stderr.read(&mut tmp) {
                    if n == 0 {
                        break;
                    }
                    let text = String::from_utf8_lossy(&tmp[..n]);
                    for line in text.lines().filter(|l| !l.trim().is_empty()) {
                        tracing::debug!("ffmpeg: {line}");
                    }
                    if let Ok(mut g) = buf.lock() {
                        g.push_str(&text);
                        if g.len() > 16 * 1024 {
                            let drain = g.len() - 8 * 1024;
                            g.drain(..drain);
                        }
                    }
                }
            })
            .map_err(|e| format!("ffmpeg stderr thread: {e}"))?;
    }

    {
        use std::os::fd::AsRawFd;
        if let Some(ref sin) = child.stdin {
            set_pipe_size(sin.as_raw_fd(), 1 << 20);
        }
        if let Some(ref sout) = child.stdout {
            set_pipe_size(sout.as_raw_fd(), 1 << 20);
        }
    }

    let stdout = child.stdout.take().ok_or("ffmpeg stdout missing")?;
    let stdin = child.stdin.take();
    let (tx, rx) = std::sync::mpsc::channel();
    let hevc = codec.is_hevc();
    std::thread::Builder::new()
        .name(format!("ffmpeg-au-{}", codec.name()))
        .spawn(move || ffmpeg_au_reader(stdout, tx, hevc))
        .map_err(|e| format!("ffmpeg reader thread: {e}"))?;

    Ok(VideoEncoder {
        input: EncoderInput {
            backend: InputBackend::Ffmpeg {
                child,
                stdin,
                stderr: Arc::clone(&stderr_buf),
            },
            frame_len: (width * height * 3 / 2) as usize,
        },
        output: EncoderOutput {
            rx,
            stderr: Some(stderr_buf),
        },
        codec_id: if hevc { "hevc" } else { "avc" },
        param_sets: Vec::new(),
    })
}

fn nal_type(hdr: u8, hevc: bool) -> u8 {
    if hevc {
        (hdr >> 1) & 0x3f
    } else {
        hdr & 0x1f
    }
}

fn is_aud(hdr: u8, hevc: bool) -> bool {
    nal_type(hdr, hevc) == if hevc { 35 } else { 9 }
}

fn is_vcl(hdr: u8, hevc: bool) -> bool {
    let t = nal_type(hdr, hevc);
    if hevc {
        t <= 31
    } else {
        (1..=5).contains(&t)
    }
}

/// Read Annex-B from ffmpeg and emit one AU per AUD-delimited unit (AUD
/// stripped). An AU is complete when the next AUD arrives, or — for the most
/// recent one, since ffmpeg waits for the next input frame — when stdout goes
/// idle after a VCL NAL.
fn ffmpeg_au_reader(mut stdout: impl Read + std::os::fd::AsRawFd, tx: Sender<Vec<u8>>, hevc: bool) {
    const IDLE: Duration = Duration::from_millis(2);
    let fd = stdout.as_raw_fd();
    let mut buf: Vec<u8> = Vec::with_capacity(1 << 20);
    let mut tmp = vec![0u8; 1 << 20];
    let mut last_data = Instant::now();

    // Emit every AU in `buf` that is followed by another AUD.
    let drain_complete = |buf: &mut Vec<u8>, tx: &Sender<Vec<u8>>| -> bool {
        loop {
            let auds: Vec<usize> = annexb_nals(buf)
                .into_iter()
                .filter(|&(_, hdr)| is_aud(buf[hdr], hevc))
                .map(|(sc, _)| sc)
                .take(2)
                .collect();
            if auds.len() < 2 {
                // Drop junk before the first AUD.
                if let Some(&first) = auds.first() {
                    buf.drain(..first);
                }
                return true;
            }
            let au: Vec<u8> = buf.drain(..auds[1]).collect();
            if tx.send(strip_aud(&au, hevc)).is_err() {
                return false;
            }
        }
    };

    loop {
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let pr = unsafe { libc::poll(&mut pfd, 1, 1) };
        if pr < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            break;
        }
        if pr > 0 {
            match stdout.read(&mut tmp) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                    last_data = Instant::now();
                    if !drain_complete(&mut buf, &tx) {
                        return;
                    }
                }
            }
        } else if !buf.is_empty() && last_data.elapsed() >= IDLE {
            let nals = annexb_nals(&buf);
            let starts_with_aud = nals.first().is_some_and(|&(sc, hdr)| sc == 0 && is_aud(buf[hdr], hevc));
            if starts_with_aud && nals.iter().any(|&(_, hdr)| is_vcl(buf[hdr], hevc)) {
                let au = std::mem::take(&mut buf);
                if tx.send(strip_aud(&au, hevc)).is_err() {
                    return;
                }
            }
        }
    }
    if !buf.is_empty() {
        let _ = tx.send(strip_aud(&buf, hevc));
    }
}

/// Remove AUD NALs (the client never saw them from the Windows server).
fn strip_aud(au: &[u8], hevc: bool) -> Vec<u8> {
    let nals = annexb_nals(au);
    let mut out = Vec::with_capacity(au.len());
    for (i, &(sc, hdr)) in nals.iter().enumerate() {
        let end = nals.get(i + 1).map(|n| n.0).unwrap_or(au.len());
        if !is_aud(au[hdr], hevc) {
            out.extend_from_slice(&au[sc..end]);
        }
    }
    out
}

/// (start-code offset, NAL header offset) for every NAL in an Annex-B buffer.
fn annexb_nals(buf: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 3 < buf.len() {
        if buf[i] == 0 && buf[i + 1] == 0 {
            if buf[i + 2] == 1 {
                out.push((i, i + 3));
                i += 3;
                continue;
            }
            if buf[i + 2] == 0 && buf[i + 3] == 1 && i + 4 < buf.len() {
                out.push((i, i + 4));
                i += 4;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// AU contains an IDR (AVC 5, HEVC IDR_W_RADL / IDR_N_LP / CRA).
pub fn annexb_has_idr(au: &[u8], hevc: bool) -> bool {
    annexb_nals(au).iter().any(|&(_, hdr)| {
        let t = nal_type(au[hdr], hevc);
        if hevc {
            (19..=21).contains(&t)
        } else {
            t == 5
        }
    })
}

/// SPS/PPS (AVC) or VPS/SPS/PPS (HEVC) that precede the first slice.
fn extract_param_sets(au: &[u8], hevc: bool) -> Vec<u8> {
    let nals = annexb_nals(au);
    let mut out = Vec::new();
    for (i, &(sc, hdr)) in nals.iter().enumerate() {
        let end = nals.get(i + 1).map(|n| n.0).unwrap_or(au.len());
        let t = nal_type(au[hdr], hevc);
        if (hevc && (32..=34).contains(&t)) || (!hevc && (t == 7 || t == 8)) {
            out.extend_from_slice(&au[sc..end]);
        } else if is_vcl(au[hdr], hevc) {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_aud_and_finds_hevc_idr() {
        // AUD, VPS, SPS, PPS, IDR_W_RADL (HEVC headers)
        let au = [
            0, 0, 0, 1, 0x46, 0x01, 0x10, //
            0, 0, 0, 1, 0x40, 0x01, 0xaa, //
            0, 0, 0, 1, 0x42, 0x01, 0xbb, //
            0, 0, 0, 1, 0x44, 0x01, 0xcc, //
            0, 0, 0, 1, 0x26, 0x01, 0xdd, 0xee,
        ];
        let s = strip_aud(&au, true);
        assert_eq!(&s[..6], &[0, 0, 0, 1, 0x40, 0x01]);
        assert!(annexb_has_idr(&s, true));
        assert_eq!(extract_param_sets(&s, true).len(), 21);
        let p = [0u8, 0, 0, 1, 0x02, 0x01, 0xdd];
        assert!(!annexb_has_idr(&p, true));
    }

    #[test]
    fn finds_avc_idr_and_params() {
        let au = [
            0, 0, 0, 1, 0x67, 0x42, 0xaa, //
            0, 0, 0, 1, 0x68, 0xce, 0x3c, //
            0, 0, 0, 1, 0x06, 0x05, 0x01, //
            0, 0, 0, 1, 0x65, 0x88, 0x80,
        ];
        assert!(annexb_has_idr(&au, false));
        assert_eq!(extract_param_sets(&au, false).len(), 14);
        assert!(!annexb_has_idr(&[0, 0, 0, 1, 0x41, 0x9a, 0x00], false));
    }
}
