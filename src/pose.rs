//! Shared HMD / controller pose and input state from ReliveVR DeviceEvents.
//! Used by the UDP server and the optional OpenGL visualizer.

use std::collections::HashMap;


// --- Pose / DeviceEvent (cap2 + live) ---

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PoseVal {
    #[serde(default)]
    orient: Option<[f64; 4]>,
    #[serde(default)]
    pos: Option<[f64; 3]>,
    #[serde(default)]
    #[serde(rename = "baseFrmIdx")]
    #[allow(dead_code)]
    base_frm_idx: Option<u64>,
    #[serde(default)]
    #[serde(rename = "frmIdx")]
    frm_idx: Option<u64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PoseSample {
    #[serde(default)]
    time: Option<u64>,
    #[serde(default)]
    val: serde_json::Value,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PoseEvent {
    id: String,
    #[serde(default)]
    data: Vec<PoseSample>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DeviceEventMsg {
    #[serde(default)]
    events: Vec<PoseEvent>,
}

/// One tracked body (HMD or controller) in OpenXR-ish form.
#[derive(Debug, Clone, Default)]
pub struct TrackedPose {
    pub orient: [f32; 4], // qx,qy,qz,qw
    pub pos: [f32; 3],    // x,y,z metres
    #[allow(dead_code)]
    time: u64,
    #[allow(dead_code)]
    frm_idx: u64,
}

/// One digital or analog input path (click, trigger, trackpad, …).
#[derive(Debug, Clone)]
pub struct InputSample {
    /// Path id from DeviceEvent, e.g. `/ctrlRight/in/vol/+/click`.
    #[allow(dead_code)]
    id: String,
    /// True for boolean clicks / touch; for axes derived from non-zero magnitude.
    pub pressed: bool,
    /// 1D analog (trigger) or first component of 2D.
    pub axis: Option<f32>,
    /// Second component when present (trackpad / joystick `[x,y]`).
    pub axis_y: Option<f32>,
    #[allow(dead_code)]
    time: u64,
}

/// Latest poses + batteries + controller inputs (shared for future OpenXR feed).
#[derive(Debug, Clone, Default)]
pub struct LatestPoses {
    pub hmd: Option<TrackedPose>,
    pub ctrl_right: Option<TrackedPose>,
    hmd_battery: Option<f32>,
    ctrl_right_battery: Option<f32>,
    /// Last known state per input path (clicks, axes).
    pub inputs: HashMap<String, InputSample>,
    /// Count of empty `{}` DeviceEvents (Daydream system / app button).
    pub system_clicks: u64,
    pub updates: u64,
    /// Non-pose input events since start (for rate-limited logging).
    pub input_events: u64,
}

impl LatestPoses {
    pub fn apply_device_event(&mut self, msg: &DeviceEventMsg) {
        self.updates = self.updates.wrapping_add(1);
        for ev in &msg.events {
            let sample = match ev.data.first() {
                Some(s) => s,
                None => continue,
            };
            if ev.id.ends_with("/battery") {
                if let Some(v) = sample.val.as_f64() {
                    let b = v as f32;
                    if ev.id.starts_with("/hmd") {
                        self.hmd_battery = Some(b);
                    } else if ev.id.contains("ctrlRight") || ev.id.contains("ctrl") {
                        self.ctrl_right_battery = Some(b);
                    }
                }
                continue;
            }
            // Pose paths carry orient/pos objects.
            if let Ok(pv) = serde_json::from_value::<PoseVal>(sample.val.clone()) {
                if let (Some(o), Some(p)) = (pv.orient, pv.pos) {
                    let tp = TrackedPose {
                        orient: [o[0] as f32, o[1] as f32, o[2] as f32, o[3] as f32],
                        pos: [p[0] as f32, p[1] as f32, p[2] as f32],
                        time: sample.time.unwrap_or(0),
                        frm_idx: pv.frm_idx.unwrap_or(0),
                    };
                    if ev.id == "/hmd/pose" || ev.id.ends_with("/hmd/pose") {
                        self.hmd = Some(tp);
                    } else if ev.id.contains("ctrlRight") || ev.id.contains("/ctrl") {
                        self.ctrl_right = Some(tp);
                    }
                    continue;
                }
            }
            // Boolean / scalar / 2D inputs: /in/…/click, /in/tp/val=[x,y], …
            // Live Daydream (2026-10-09):
            //   /ctrlRight/in/tp/val   → [x,y] floats ≈ −1..1
            //   /ctrlRight/in/tp/click → bool
            //   /ctrlRight/in/tp/touch → bool
            if ev.id.contains("/in/") {
                let (pressed, axis, axis_y) = match &sample.val {
                    serde_json::Value::Bool(b) => (*b, None, None),
                    serde_json::Value::Number(n) => {
                        let f = n.as_f64().unwrap_or(0.0) as f32;
                        (f != 0.0, Some(f), None)
                    }
                    serde_json::Value::Array(a) if a.len() >= 2 => {
                        let x = a[0].as_f64().unwrap_or(0.0) as f32;
                        let y = a[1].as_f64().unwrap_or(0.0) as f32;
                        (x != 0.0 || y != 0.0, Some(x), Some(y))
                    }
                    serde_json::Value::Array(a) if !a.is_empty() => {
                        let f = a[0].as_f64().unwrap_or(0.0) as f32;
                        (f != 0.0, Some(f), None)
                    }
                    _ => (false, None, None),
                };
                self.inputs.insert(
                    ev.id.clone(),
                    InputSample {
                        id: ev.id.clone(),
                        pressed,
                        axis,
                        axis_y,
                        time: sample.time.unwrap_or(0),
                    },
                );
                self.input_events = self.input_events.wrapping_add(1);
            }
        }
    }

    pub fn note_system_click(&mut self) {
        self.system_clicks = self.system_clicks.wrapping_add(1);
        self.input_events = self.input_events.wrapping_add(1);
    }

    pub fn summary_line(&self) -> String {
        let mut parts = Vec::new();
        if let Some(h) = &self.hmd {
            parts.push(format!(
                "/hmd q=[{:.3},{:.3},{:.3},{:.3}] p=[{:.3},{:.3},{:.3}]",
                h.orient[0], h.orient[1], h.orient[2], h.orient[3],
                h.pos[0], h.pos[1], h.pos[2]
            ));
        }
        if let Some(c) = &self.ctrl_right {
            parts.push(format!(
                "/ctrlRight q=[{:.3},{:.3},{:.3},{:.3}] p=[{:.3},{:.3},{:.3}]",
                c.orient[0], c.orient[1], c.orient[2], c.orient[3],
                c.pos[0], c.pos[1], c.pos[2]
            ));
        }
        if let Some(b) = self.hmd_battery {
            parts.push(format!("hmd_bat={:.2}", b));
        }
        if let Some(b) = self.ctrl_right_battery {
            parts.push(format!("ctrl_bat={:.2}", b));
        }
        // Pressed digital inputs + non-zero axes (compact path tails).
        let mut pressed: Vec<String> = Vec::new();
        for (id, s) in &self.inputs {
            if !s.pressed {
                continue;
            }
            let short: String = id
                .rsplit('/')
                .take(3)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("/");
            match (s.axis, s.axis_y) {
                (Some(x), Some(y)) => {
                    pressed.push(format!("{}=[{:.2},{:.2}]", short, x, y));
                }
                (Some(a), None) if (a - 1.0).abs() > 1e-3 && a.abs() > 1e-3 => {
                    pressed.push(format!("{}={:.2}", short, a));
                }
                _ => pressed.push(short),
            }
        }
        if !pressed.is_empty() {
            parts.push(format!("in=[{}]", pressed.join(",")));
        }
        if self.system_clicks > 0 {
            parts.push(format!("sys_click={}", self.system_clicks));
        }
        if parts.is_empty() {
            format!("updates={}", self.updates)
        } else {
            parts.join(" | ")
        }
    }

    /// True if this event is only continuous axis motion (rate-limit logs).
    pub fn is_axis_only_event(msg: &DeviceEventMsg) -> bool {
        let mut any = false;
        for ev in &msg.events {
            if !ev.id.contains("/in/") {
                continue;
            }
            any = true;
            // Discrete edges (click/touch/button) always worth logging.
            if ev.id.ends_with("/click")
                || ev.id.ends_with("/touch")
                || ev.id.contains("/vol/")
                || ev.id.contains("/sys/")
                || ev.id.contains("/menu/")
            {
                return false;
            }
        }
        any
    }

    /// One-line for a single non-pose input event (immediate log).
    pub fn describe_input_event(msg: &DeviceEventMsg) -> Option<String> {
        let mut bits = Vec::new();
        for ev in &msg.events {
            if !ev.id.contains("/in/") {
                continue;
            }
            let sample = match ev.data.first() {
                Some(s) => s,
                None => continue,
            };
            let val_s = match &sample.val {
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => format!("{:.3}", n.as_f64().unwrap_or(0.0)),
                serde_json::Value::Array(a) if a.len() >= 2 => {
                    format!(
                        "[{:.3},{:.3}]",
                        a[0].as_f64().unwrap_or(0.0),
                        a[1].as_f64().unwrap_or(0.0)
                    )
                }
                serde_json::Value::Array(a) if !a.is_empty() => {
                    format!("{:.3}", a[0].as_f64().unwrap_or(0.0))
                }
                other => other.to_string(),
            };
            bits.push(format!("{}={}", ev.id, val_s));
        }
        if bits.is_empty() {
            None
        } else {
            Some(bits.join(" "))
        }
    }
}

/// Compact one-line summary from a raw DeviceEvent (fallback).
#[allow(dead_code)]
pub fn summarize_pose(msg: &DeviceEventMsg) -> String {
    let mut tmp = LatestPoses::default();
    tmp.apply_device_event(msg);
    tmp.summary_line()
}

