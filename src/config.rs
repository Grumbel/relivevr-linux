//! Command line options. Every option can also be set by its `RELIVEVR_*`
//! environment variable; the command line wins.

use std::sync::OnceLock;

use clap::builder::FalseyValueParser;
use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about = "ReliveVR (AMD Wireless GVR) protocol test server for Linux")]
pub struct Config {
    /// OpenGL window with the pose visualizer, streamed live to the headset
    #[arg(long, env = "RELIVEVR_VIZ", value_parser = FalseyValueParser::new())]
    pub viz: bool,

    /// Encoder backend: auto, ffmpeg, vaapi, nvenc, qsv, x264, openh264, hevc,
    /// hevc_vaapi, hevc_nvenc
    #[arg(long, env = "RELIVEVR_ENCODER", default_value = "auto")]
    pub encoder: String,

    /// ffmpeg binary
    #[arg(long, env = "RELIVEVR_FFMPEG", default_value = "ffmpeg")]
    pub ffmpeg: String,

    /// VAAPI render node
    #[arg(long, env = "RELIVEVR_VAAPI_DEVICE", default_value = "/dev/dri/renderD128")]
    pub vaapi_device: String,

    /// Encode width per eye (rounded up to even)
    #[arg(long, env = "RELIVEVR_ENCODE_W", default_value_t = 1440)]
    pub width: u32,

    /// Encode height per eye (rounded up to even)
    #[arg(long, env = "RELIVEVR_ENCODE_H", default_value_t = 1440)]
    pub height: u32,

    /// Encode/send rate and pts step (Mirage Solo display: 75)
    #[arg(long, env = "RELIVEVR_ENCODE_FPS", default_value_t = 75.0)]
    pub fps: f32,

    /// Target bitrate per eye in bps
    #[arg(long, env = "RELIVEVR_ENCODE_BITRATE", default_value_t = 10_000_000)]
    pub bitrate: u32,

    /// IDR interval in frames [default: fps, about 1 s]
    #[arg(long, env = "RELIVEVR_ENCODE_GOP")]
    pub gop: Option<u32>,

    /// Per-eye FOV in degrees: advertised in Hello and used for rendering
    #[arg(long, env = "RELIVEVR_FOV_DEG", default_value_t = 100.0,
          value_parser = parse_fov)]
    pub fov: f32,

    /// Low-fps simulation: render the scene and sample the pose at this rate only
    #[arg(long, env = "RELIVEVR_SIM_FPS")]
    pub sim_fps: Option<f32>,

    /// What the stream does between simulated frames
    #[arg(long, env = "RELIVEVR_SIM_MODE", value_enum, default_value_t = SimMode::Drop)]
    pub sim_mode: SimMode,

    /// Which HMD pose VideoData ptsSensor names
    #[arg(long, env = "RELIVEVR_PTS_SENSOR", value_enum, default_value_t = PtsSensor::Render)]
    pub pts_sensor: PtsSensor,

    /// HelloResponse style
    #[arg(long, env = "RELIVEVR_STYLE", value_enum, default_value_t = HelloStyle::Full)]
    pub style: HelloStyle,

    /// Type byte of the HelloResponse [default: 0]
    #[arg(long, env = "RELIVEVR_TYPE")]
    pub hello_type: Option<u8>,

    /// Type byte of VideoInit [default: 2]
    #[arg(long, env = "RELIVEVR_VIDEOINIT_TYPE")]
    pub videoinit_type: Option<u8>,

    /// Broadcast HelloResponse on the LAN every second
    #[arg(long, env = "RELIVEVR_ANNOUNCE", value_parser = FalseyValueParser::new())]
    pub announce: bool,
}

/// What the stream does between two simulated frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum SimMode {
    /// Send nothing: the stream runs at --sim-fps, pts keeps stepping at the
    /// encode rate (like the ALVR fork with a slow game)
    Drop,
    /// Re-encode the last rendered image at the encode rate, with its original
    /// pose time as ptsSensor
    Repeat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum PtsSensor {
    /// The pose the frame was rendered with
    Render,
    /// The newest HMD pose at send time (old tip 087.1); the client's
    /// reprojection then corrects from the wrong pose
    Latest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum HelloStyle {
    Minimal,
    Echo,
    Full,
}

fn parse_fov(s: &str) -> Result<f32, String> {
    let f: f32 = s.parse().map_err(|e| format!("{e}"))?;
    if (30.0..170.0).contains(&f) {
        Ok(f)
    } else {
        Err("must be between 30 and 170".into())
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

/// Parse the command line (exits on --help or an error). Call once from main.
pub fn init() -> &'static Config {
    CONFIG.get_or_init(Config::parse)
}

/// The parsed options; the defaults if `init` was never called.
pub fn get() -> &'static Config {
    CONFIG.get_or_init(|| Config::parse_from(["relivevr-server"]))
}
