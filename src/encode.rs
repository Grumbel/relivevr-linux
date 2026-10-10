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


#[cfg(target_os = "linux")]
fn set_pipe_size(fd: i32, bytes: i32) {
    // Best-effort: larger pipes avoid stdin-write / stdout-read deadlocks on
    // big NV12 frames (1440² ≈ 3.1 MiB).
    unsafe {
        let _ = libc::fcntl(fd, libc::F_SETPIPE_SZ, bytes);
    }
}

#[cfg(not(target_os = "linux"))]
fn set_pipe_size(_fd: i32, _bytes: i32) {}

struct FfmpegEncoder {
    child: Child,
    /// Pre-allocated NV12 staging buffer.
    nv12: Vec<u8>,
    #[allow(dead_code)]
    codec: FfmpegCodec,
    width: u32,
    height: u32,
    fps: f32,
    bitrate: u32,
    gop: u32,
    /// Access units from a dedicated stdout reader thread.
    au_rx: std::sync::mpsc::Receiver<Vec<u8>>,
    /// Captured stderr (shared with a drain thread).
    stderr_buf: Arc<Mutex<String>>,
    /// Frames encoded successfully (used to widen first-frame timeout).
    ok_frames: u64,
    /// When true, each encode_rgba spawns a fresh ffmpeg (-frames:v 1).
    /// Used when the persistent pipe refuses to emit AUs.
    oneshot: bool,
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
        let bufsize = (bitrate / 2).max(1).to_string();

        // Feed raw frames on stdin (pipe:0). Encode as fast as frames arrive
        // (fps_mode passthrough) — do not wall-clock pace the pipe.
        let mut args: Vec<String> = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "warning".into(),
            "-fflags".into(),
            "+nobuffer+flush_packets".into(),
            "-flags".into(),
            "low_delay".into(),
        ];

        if matches!(codec, FfmpegCodec::Vaapi) {
            let dev = std::env::var("RELIVEVR_VAAPI_DEVICE")
                .unwrap_or_else(|_| "/dev/dri/renderD128".into());
            args.push("-vaapi_device".into());
            args.push(dev);
        }

        args.extend([
            "-f".into(),
            "rawvideo".into(),
            "-pix_fmt".into(),
            "nv12".into(),
            "-s:v".into(),
            size.clone(),
            "-r".into(),
            fps_s,
            "-i".into(),
            "pipe:0".into(),
        ]);

        match codec {
            FfmpegCodec::Vaapi => {
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
                // Consumer NVENC: main profile (baseline often rejected), low-latency tune.
                // Avoid -zerolatency (x264-style) and exotic rc modes that abort on some drivers.
                args.extend([
                    "-c:v".into(),
                    "h264_nvenc".into(),
                    "-preset".into(),
                    "p1".into(),
                    "-tune".into(),
                    "ll".into(),
                    "-profile:v".into(),
                    "main".into(),
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
                    "-delay".into(),
                    "0".into(),
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
                    "-x264-params".into(),
                    "annexb=1:sliced-threads=0:sync-lookahead=0:rc-lookahead=0".into(),
                ]);
            }
        }

        args.extend([
            "-an".into(),
            "-fps_mode".into(),
            "passthrough".into(),
            "-flush_packets".into(),
            "1".into(),
            "-f".into(),
            "h264".into(),
            "pipe:1".into(),
        ]);

        tracing::debug!(
            "ffmpeg cmdline: {} {}",
            bin,
            args.iter().map(|s| {
                if s.contains(' ') { format!("'{s}'") } else { s.clone() }
            }).collect::<Vec<_>>().join(" ")
        );

        let mut child = Command::new(&bin)
            .args(&args)
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
                    loop {
                        match stderr.read(&mut tmp) {
                            Ok(0) => break,
                            Ok(n) => {
                                if let Ok(mut g) = buf.lock() {
                                    g.push_str(&String::from_utf8_lossy(&tmp[..n]));
                                    // Cap stderr capture
                                    if g.len() > 16 * 1024 {
                                        let drain = g.len() - 8 * 1024;
                                        g.drain(..drain);
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                })
                .map_err(|e| format!("ffmpeg stderr thread: {e}"))?;
        }

        // Brief settle — some hw drivers abort immediately on bad options.
        std::thread::sleep(std::time::Duration::from_millis(30));
        if let Ok(Some(status)) = child.try_wait() {
            let err = stderr_buf.lock().map(|g| g.clone()).unwrap_or_default();
            return Err(format!(
                "{bin} {} exited immediately ({status}): {err}",
                codec.name()
            ));
        }

        // Enlarge pipe buffers before any large NV12 writes (Linux).
        {
            use std::os::fd::AsRawFd;
            if let Some(ref sin) = child.stdin {
                set_pipe_size(sin.as_raw_fd(), 4 * 1024 * 1024);
            }
            if let Some(ref sout) = child.stdout {
                set_pipe_size(sout.as_raw_fd(), 4 * 1024 * 1024);
            }
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "ffmpeg stdout missing".to_string())?;
        let (au_tx, au_rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let codec_name = codec.name();
        std::thread::Builder::new()
            .name(format!("ffmpeg-au-{codec_name}"))
            .spawn(move || {
                ffmpeg_au_reader(stdout, au_tx);
            })
            .map_err(|e| format!("ffmpeg reader thread: {e}"))?;

        let nv12_len = (width as usize) * (height as usize) * 3 / 2;
        let mut enc = Self {
            child,
            nv12: vec![0u8; nv12_len], // black / zero UV mid-gray-ish
            codec,
            width,
            height,
            fps,
            bitrate,
            gop,
            au_rx,
            stderr_buf,
            ok_frames: 0,
            oneshot: false,
        };
        // Mid-gray chroma for a valid NV12 black-ish frame
        let y_size = (width as usize) * (height as usize);
        for b in &mut enc.nv12[y_size..] {
            *b = 128;
        }

        // Warm-up: one frame → wait for AU, up to 3 tries.
        // Writing multiple large NV12 frames before reading stdout can fill the
        // OS pipe and deadlock (stdin write blocks, stdout unread).
        let mut warm_ok = false;
        let mut last_err = String::from("no attempt");
        for i in 0..3 {
            // Write on a helper thread so a large NV12 write cannot stall this
            // thread while the AU reader needs CPU time.
            let nv12 = enc.nv12.clone();
            // Take stdin briefly — ChildStdin is Write; we re-wrap after join.
            let mut stdin = enc
                .child
                .stdin
                .take()
                .ok_or("ffmpeg stdin closed during warm-up")?;
            let write_handle = std::thread::Builder::new()
                .name(format!("ffmpeg-wu-{}", codec.name()))
                .spawn(move || {
                    stdin.write_all(&nv12)?;
                    stdin.flush()?;
                    Ok::<_, std::io::Error>(stdin)
                })
                .map_err(|e| format!("warm-up write thread: {e}"))?;

            match enc.wait_au(std::time::Duration::from_millis(2000)) {
                Ok(au) => {
                    // Re-attach stdin for later encodes.
                    match write_handle.join() {
                        Ok(Ok(s)) => enc.child.stdin = Some(s),
                        Ok(Err(e)) => {
                            last_err = format!("warm-up write: {e}");
                            warn!("FFmpeg {} warm-up try {i} write err: {last_err}", codec.name());
                            continue;
                        }
                        Err(_) => {
                            last_err = "warm-up write thread panicked".into();
                            continue;
                        }
                    }
                    info!(
                        "FFmpeg {} ready {}x{} bitrate={bitrate} fps={fps:.0} gop={gop} (warm-up OK, AU {}B try {i})",
                        codec.name(),
                        width,
                        height,
                        au.len()
                    );
                    warm_ok = true;
                    break;
                }
                Err(e) => {
                    last_err = e;
                    match write_handle.join() {
                        Ok(Ok(s)) => enc.child.stdin = Some(s),
                        Ok(Err(we)) => last_err = format!("{last_err}; write: {we}"),
                        Err(_) => {}
                    }
                    warn!(
                        "FFmpeg {} warm-up try {i} failed: {last_err}",
                        codec.name()
                    );
                }
            }
        }
        if !warm_ok {
            let err = enc.stderr_snapshot();
            warn!(
                "FFmpeg {} persistent pipe warm-up failed ({last_err}; stderr: {err}); trying oneshot mode",
                codec.name()
            );
            // Kill the silent process and switch to per-frame spawn.
            enc.shutdown();
            // Validate oneshot with one black frame.
            match oneshot_encode_nv12(
                &enc.nv12,
                width,
                height,
                fps,
                bitrate,
                gop,
                codec,
            ) {
                Ok(au) if !au.is_empty() => {
                    info!(
                        "FFmpeg {} oneshot mode ready {}x{} (probe AU {}B)",
                        codec.name(),
                        width,
                        height,
                        au.len()
                    );
                    enc.oneshot = true;
                    // Re-spawn a dummy child so Drop/shutdown stay safe (will be unused).
                    // Leave child already waited; encode_rgba oneshot path ignores it.
                }
                Ok(_) => {
                    return Err(format!(
                        "FFmpeg {} oneshot probe returned empty AU",
                        codec.name()
                    ));
                }
                Err(e) => {
                    return Err(format!(
                        "FFmpeg {} pipe and oneshot both failed: pipe={last_err}; oneshot={e}",
                        codec.name()
                    ));
                }
            }
        }

        Ok(enc)
    }

    fn stderr_snapshot(&self) -> String {
        self.stderr_buf
            .lock()
            .map(|g| g.trim().to_string())
            .unwrap_or_default()
    }

    fn wait_au(&mut self, timeout: std::time::Duration) -> Result<Vec<u8>, String> {
        let deadline = Instant::now() + timeout;
        let mut last = None;
        loop {
            let slice = deadline.saturating_duration_since(Instant::now());
            if slice.is_zero() {
                if let Some(au) = last {
                    return Ok(au);
                }
                if let Ok(Some(st)) = self.child.try_wait() {
                    return Err(format!(
                        "exited ({st}); stderr: {}",
                        self.stderr_snapshot()
                    ));
                }
                return Err(format!(
                    "AU timeout; stderr: {}",
                    self.stderr_snapshot()
                ));
            }
            match self.au_rx.recv_timeout(std::time::Duration::from_millis(5).min(slice)) {
                Ok(au) => {
                    last = Some(au);
                    while let Ok(au) = self.au_rx.try_recv() {
                        last = Some(au);
                    }
                    // Prefer returning as soon as we have one AU.
                    return Ok(last.take().unwrap());
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(format!(
                        "reader disconnected; stderr: {}",
                        self.stderr_snapshot()
                    ));
                }
            }
        }
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

        if self.oneshot {
            let nals = oneshot_encode_nv12(
                &self.nv12,
                self.width,
                self.height,
                self.fps,
                self.bitrate,
                self.gop,
                self.codec,
            )?;
            self.ok_frames = self.ok_frames.saturating_add(1);
            let is_idr = annexb_has_idr(&nals);
            return Ok((nals, is_idr));
        }

        // Write without holding a second borrow of `self` for error formatting.
        let write_result = {
            match self.child.stdin.as_mut() {
                None => Err("ffmpeg stdin closed".to_string()),
                Some(stdin) => stdin
                    .write_all(&self.nv12)
                    .and_then(|_| stdin.flush())
                    .map_err(|e| format!("ffmpeg stdin write/flush: {e}")),
            }
        };
        if let Err(e) = write_result {
            return Err(format!("{e}; stderr: {}", self.stderr_snapshot()));
        }

        // First frames can take longer while the encoder finishes initialising.
        let timeout = if self.ok_frames < 3 {
            std::time::Duration::from_millis(1500)
        } else {
            std::time::Duration::from_millis(250)
        };
        let nals = self.wait_au(timeout)?;
        self.ok_frames = self.ok_frames.saturating_add(1);
        let is_idr = annexb_has_idr(&nals);
        Ok((nals, is_idr))
    }

    fn shutdown(&mut self) {
        let _ = self.child.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for FfmpegEncoder {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Continuously read Annex-B from ffmpeg stdout and push complete access units.

/// Encode exactly one NV12 frame via a fresh ffmpeg process (stdin → close → stdout).
/// Reliable but slower; used when the persistent pipe never emits AUs.
fn oneshot_encode_nv12(
    nv12: &[u8],
    width: u32,
    height: u32,
    fps: f32,
    bitrate: u32,
    gop: u32,
    codec: FfmpegCodec,
) -> Result<Vec<u8>, String> {
    let bin = ffmpeg_bin();
    let size = format!("{width}x{height}");
    let br = bitrate.to_string();
    let gop_s = gop.to_string();
    let fps_s = format!("{fps:.3}");

    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        "nv12".into(),
        "-s:v".into(),
        size,
        "-r".into(),
        fps_s,
        "-i".into(),
        "pipe:0".into(),
        "-frames:v".into(),
        "1".into(),
    ];
    match codec {
        FfmpegCodec::Nvenc => {
            args.extend([
                "-c:v".into(), "h264_nvenc".into(),
                "-preset".into(), "p1".into(),
                "-tune".into(), "ll".into(),
                "-profile:v".into(), "main".into(),
                "-bf".into(), "0".into(),
                "-g".into(), gop_s,
                "-b:v".into(), br,
            ]);
        }
        FfmpegCodec::Vaapi => {
            let dev = std::env::var("RELIVEVR_VAAPI_DEVICE")
                .unwrap_or_else(|_| "/dev/dri/renderD128".into());
            // vaapi needs device before -i ideally; still try
            args = vec![
                "-hide_banner".into(), "-loglevel".into(), "error".into(),
                "-vaapi_device".into(), dev,
                "-f".into(), "rawvideo".into(),
                "-pix_fmt".into(), "nv12".into(),
                "-s:v".into(), format!("{width}x{height}"),
                "-i".into(), "pipe:0".into(),
                "-frames:v".into(), "1".into(),
                "-vf".into(), "format=nv12,hwupload".into(),
                "-c:v".into(), "h264_vaapi".into(),
                "-bf".into(), "0".into(),
                "-b:v".into(), br,
            ];
        }
        FfmpegCodec::Qsv => {
            args.extend([
                "-c:v".into(), "h264_qsv".into(),
                "-bf".into(), "0".into(),
                "-b:v".into(), br,
            ]);
        }
        FfmpegCodec::X264 => {
            args.extend([
                "-c:v".into(), "libx264".into(),
                "-preset".into(), "ultrafast".into(),
                "-tune".into(), "zerolatency".into(),
                "-profile:v".into(), "baseline".into(),
                "-bf".into(), "0".into(),
                "-g".into(), gop_s,
                "-b:v".into(), br,
                "-x264-params".into(),
                "annexb=1:sliced-threads=0:sync-lookahead=0:rc-lookahead=0".into(),
            ]);
        }
    }
    args.extend(["-an".into(), "-f".into(), "h264".into(), "pipe:1".into()]);

    let mut child = Command::new(&bin)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("oneshot spawn {bin}: {e}"))?;

    let mut stdin = child.stdin.take().ok_or("oneshot stdin missing")?;
    let mut stdout = child.stdout.take().ok_or("oneshot stdout missing")?;
    let stderr = child.stderr.take();

    let nv12 = nv12.to_vec();
    let writer = std::thread::spawn(move || {
        stdin.write_all(&nv12)?;
        // Closing stdin signals EOF → ffmpeg finishes the single frame.
        drop(stdin);
        Ok::<_, std::io::Error>(())
    });

    let mut out = Vec::new();
    stdout
        .read_to_end(&mut out)
        .map_err(|e| format!("oneshot read: {e}"))?;
    let _ = writer.join();
    let status = child.wait().map_err(|e| format!("oneshot wait: {e}"))?;
    if !status.success() {
        let err = stderr
            .map(|mut s| {
                let mut e = String::new();
                let _ = s.read_to_string(&mut e);
                e
            })
            .unwrap_or_default();
        return Err(format!("oneshot ffmpeg exited {status}: {err}"));
    }
    if out.is_empty() {
        return Err("oneshot ffmpeg produced no output".into());
    }
    Ok(out)
}

fn ffmpeg_au_reader(mut stdout: impl Read + std::os::fd::AsRawFd, tx: std::sync::mpsc::Sender<Vec<u8>>) {
    // Non-blocking + poll. Idle-flush ANY buffered data (not only VCL) so we
    // never sit on SPS/PPS-only or unusual NAL layouts during warm-up.
    let fd = stdout.as_raw_fd();
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags >= 0 {
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
    }

    let mut buf = Vec::with_capacity(256 * 1024);
    let mut tmp = [0u8; 65536];
    let mut last_data = std::time::Instant::now();
    let mut total_rx: u64 = 0;

    loop {
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let pr = unsafe { libc::poll(&mut pfd, 1, 5) };
        if pr < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            break;
        }

        if pr > 0 && (pfd.revents & libc::POLLIN) != 0 {
            match stdout.read(&mut tmp) {
                Ok(0) => {
                    if !buf.is_empty() {
                        let _ = tx.send(std::mem::take(&mut buf));
                    }
                    break;
                }
                Ok(n) => {
                    total_rx = total_rx.saturating_add(n as u64);
                    buf.extend_from_slice(&tmp[..n]);
                    last_data = std::time::Instant::now();
                    while let Some(au) = pop_complete_au(&mut buf) {
                        if tx.send(au).is_err() {
                            return;
                        }
                    }
                    // Short read: treat as end of this write from ffmpeg.
                    if n < tmp.len() && !buf.is_empty() {
                        if tx.send(std::mem::take(&mut buf)).is_err() {
                            return;
                        }
                    }
                    if buf.len() > 4 * 1024 * 1024 {
                        buf.drain(..buf.len() / 2);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        } else {
            // Idle flush: any buffered data after a few ms.
            if !buf.is_empty() && last_data.elapsed() > std::time::Duration::from_millis(5) {
                if tx.send(std::mem::take(&mut buf)).is_err() {
                    return;
                }
            }
            if pr > 0 && (pfd.revents & (libc::POLLHUP | libc::POLLERR)) != 0 {
                if !buf.is_empty() {
                    let _ = tx.send(std::mem::take(&mut buf));
                }
                break;
            }
        }
    }
    if total_rx == 0 {
        tracing::debug!("ffmpeg AU reader exited with 0 bytes received");
    }
}

/// Pop one complete access unit when a *following* start code delimits a VCL NAL.
fn pop_complete_au(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    let starts = annexb_start_indices(buf);
    if starts.len() < 2 {
        return None;
    }
    for i in 0..starts.len() - 1 {
        let (pos, sc) = starts[i];
        let nal_hdr = pos + sc;
        if nal_hdr >= buf.len() {
            return None;
        }
        let nt = buf[nal_hdr] & 0x1f;
        if nt == 1 || nt == 5 {
            let end = starts[i + 1].0;
            let au = buf[..end].to_vec();
            buf.drain(..end);
            return Some(au);
        }
    }
    None
}

fn annexb_start_indices(buf: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 3 < buf.len() {
        if buf[i..].starts_with(&[0, 0, 0, 1]) {
            out.push((i, 4));
            i += 4;
        } else if buf[i..].starts_with(&[0, 0, 1]) {
            out.push((i, 3));
            i += 3;
        } else {
            i += 1;
        }
    }
    out
}

fn annexb_has_vcl(nals: &[u8]) -> bool {
    let mut i = 0;
    while i + 4 < nals.len() {
        if nals[i..].starts_with(&[0, 0, 0, 1]) {
            let nt = nals[i + 4] & 0x1f;
            if nt == 1 || nt == 5 {
                return true;
            }
            i += 4;
        } else if nals[i..].starts_with(&[0, 0, 1]) {
            let nt = nals[i + 3] & 0x1f;
            if nt == 1 || nt == 5 {
                return true;
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    false
}

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
    // Capture SPS/PPS from the first bitstream that carries them (usually the
    // first IDR). Do not require is_idr so a seed frame still works if the
    // encoder emits parameter sets on a non-IDR AU.
    if g.param_sets.is_empty() {
        let ps = extract_param_sets(&left);
        if ps.is_empty() {
            let ps_r = extract_param_sets(&right);
            if !ps_r.is_empty() {
                g.param_sets = ps_r;
            }
        } else {
            g.param_sets = ps;
        }
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
