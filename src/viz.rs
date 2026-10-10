//! Optional OpenGL window visualizing HMD + controller poses from `LatestPoses`.
//!
//! Enable with `RELIVEVR_VIZ=1`. The event loop must run on the **main** thread
//! (winit requirement). The UDP server then runs on a background tokio runtime.
//! Later this path will render to an FBO for realtime encode.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use glow::HasContext;
use tracing::info;

use crate::encode::{self, encode_dims, H264Encoder, LiveVideoSlot, target_encode_fps};
use crate::pose::{InputSample, LatestPoses, TrackedPose};

const VS: &str = r#"#version 330 core
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_col;
uniform mat4 u_mvp;
out vec3 v_col;
void main() {
    v_col = a_col;
    gl_Position = u_mvp * vec4(a_pos, 1.0);
}
"#;

const FS: &str = r#"#version 330 core
in vec3 v_col;
out vec4 frag;
void main() {
    frag = vec4(v_col, 1.0);
}
"#;

/// Column-major 4×4 matrix.
#[derive(Clone, Copy)]
struct Mat4(pub [f32; 16]);

impl Mat4 {
    fn identity() -> Self {
        Self([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ])
    }

    fn mul(self, o: Mat4) -> Mat4 {
        let mut r = [0.0f32; 16];
        for col in 0..4 {
            for row in 0..4 {
                r[col * 4 + row] = (0..4)
                    .map(|k| self.0[k * 4 + row] * o.0[col * 4 + k])
                    .sum();
            }
        }
        Mat4(r)
    }

    fn perspective(fovy_rad: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fovy_rad * 0.5).tan();
        let mut m = [0.0f32; 16];
        m[0] = f / aspect;
        m[5] = f;
        m[10] = (far + near) / (near - far);
        m[11] = -1.0;
        m[14] = (2.0 * far * near) / (near - far);
        Mat4(m)
    }

    fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> Self {
        let f = normalize([
            target[0] - eye[0],
            target[1] - eye[1],
            target[2] - eye[2],
        ]);
        let s = normalize(cross(f, up));
        let u = cross(s, f);
        Mat4([
            s[0],
            u[0],
            -f[0],
            0.0,
            s[1],
            u[1],
            -f[1],
            0.0,
            s[2],
            u[2],
            -f[2],
            0.0,
            -dot(s, eye),
            -dot(u, eye),
            dot(f, eye),
            1.0,
        ])
    }

    /// Quaternion (x,y,z,w) + translation → model matrix.
    fn from_quat_pos(q: [f32; 4], p: [f32; 3]) -> Self {
        let [x, y, z, w] = q;
        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let xy = x * y;
        let xz = x * z;
        let yz = y * z;
        let wx = w * x;
        let wy = w * y;
        let wz = w * z;
        Mat4([
            1.0 - 2.0 * (yy + zz),
            2.0 * (xy + wz),
            2.0 * (xz - wy),
            0.0,
            2.0 * (xy - wz),
            1.0 - 2.0 * (xx + zz),
            2.0 * (yz + wx),
            0.0,
            2.0 * (xz + wy),
            2.0 * (yz - wx),
            1.0 - 2.0 * (xx + yy),
            0.0,
            p[0],
            p[1],
            p[2],
            1.0,
        ])
    }

    fn inverse_rigid(self) -> Mat4 {
        let mut r = [0.0f32; 16];
        r[0] = self.0[0];
        r[1] = self.0[4];
        r[2] = self.0[8];
        r[4] = self.0[1];
        r[5] = self.0[5];
        r[6] = self.0[9];
        r[8] = self.0[2];
        r[9] = self.0[6];
        r[10] = self.0[10];
        r[15] = 1.0;
        let tx = self.0[12];
        let ty = self.0[13];
        let tz = self.0[14];
        r[12] = -(r[0] * tx + r[4] * ty + r[8] * tz);
        r[13] = -(r[1] * tx + r[5] * ty + r[9] * tz);
        r[14] = -(r[2] * tx + r[6] * ty + r[10] * tz);
        Mat4(r)
    }

    fn scale(s: [f32; 3]) -> Self {
        let mut m = Self::identity();
        m.0[0] = s[0];
        m.0[5] = s[1];
        m.0[10] = s[2];
        m
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-8);
    [v[0] / n, v[1] / n, v[2] / n]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn quat_rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let ux = y * v[2] - z * v[1];
    let uy = z * v[0] - x * v[2];
    let uz = x * v[1] - y * v[0];
    let vx = w * ux + (y * uz - z * uy);
    let vy = w * uy + (z * ux - x * uz);
    let vz = w * uz + (x * uy - y * ux);
    [v[0] + 2.0 * vx, v[1] + 2.0 * vy, v[2] + 2.0 * vz]
}

fn view_from_hmd(pose: &TrackedPose, eye_sign: f32, ipd: f32) -> Mat4 {
    let half = eye_sign * ipd * 0.5;
    let offset = quat_rotate(pose.orient, [half, 0.0, 0.0]);
    let pos = [
        pose.pos[0] + offset[0],
        pose.pos[1] + offset[1],
        pose.pos[2] + offset[2],
    ];
    Mat4::from_quat_pos(pose.orient, pos).inverse_rigid()
}

fn default_orbit_view() -> Mat4 {
    Mat4::look_at([1.6, 1.4, 1.8], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0])
}

struct Mesh {

    vao: glow::VertexArray,
    vbo: glow::Buffer,
    count: i32,
}

impl Mesh {
    unsafe fn triangles(gl: &glow::Context, verts: &[[f32; 6]]) -> Self {
        // Same vertex layout as lines; draw with draw_tris.
        Self::lines(gl, verts)
    }

    unsafe fn lines(gl: &glow::Context, verts: &[[f32; 6]]) -> Self {

        let flat: Vec<f32> = verts.iter().flat_map(|v| v.iter().copied()).collect();
        let vao = gl.create_vertex_array().unwrap();
        let vbo = gl.create_buffer().unwrap();
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            std::slice::from_raw_parts(flat.as_ptr() as *const u8, flat.len() * 4),
            glow::STATIC_DRAW,
        );
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 24, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 24, 12);
        gl.bind_vertex_array(None);
        Self {
            vao,
            vbo,
            count: verts.len() as i32,
        }
    }

    unsafe fn draw_lines(&self, gl: &glow::Context) {
        gl.bind_vertex_array(Some(self.vao));
        gl.draw_arrays(glow::LINES, 0, self.count);
        gl.bind_vertex_array(None);
    }

    #[allow(dead_code)]
    unsafe fn draw_tris(&self, gl: &glow::Context) {
        gl.bind_vertex_array(Some(self.vao));
        gl.draw_arrays(glow::TRIANGLES, 0, self.count);
        gl.bind_vertex_array(None);
    }
}

fn axis_mesh() -> Vec<[f32; 6]> {
    let s = 0.5;
    vec![
        // X red
        [0.0, 0.0, 0.0, 1.0, 0.2, 0.2],
        [s, 0.0, 0.0, 1.0, 0.2, 0.2],
        // Y green
        [0.0, 0.0, 0.0, 0.2, 1.0, 0.2],
        [0.0, s, 0.0, 0.2, 1.0, 0.2],
        // Z blue
        [0.0, 0.0, 0.0, 0.2, 0.4, 1.0],
        [0.0, 0.0, s, 0.2, 0.4, 1.0],
    ]
}

/// Checkerboard room: floor, ceiling, four walls (triangle list).
fn room_mesh() -> Vec<[f32; 6]> {
    let mut v = Vec::new();
    let half = 2.5f32; // room extends ±half metres
    let height = 2.5f32;
    let cells = 10; // tiles per edge
    let step = (2.0 * half) / cells as f32;
    let light = [0.55f32, 0.55, 0.58];
    let dark = [0.18f32, 0.18, 0.22];

    let mut push_quad = |p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], col: [f32; 3]| {
        // two triangles: 0-1-2, 0-2-3
        for p in [p0, p1, p2, p0, p2, p3] {
            v.push([p[0], p[1], p[2], col[0], col[1], col[2]]);
        }
    };

    // Floor y=0 and ceiling y=height
    for iz in 0..cells {
        for ix in 0..cells {
            let x0 = -half + ix as f32 * step;
            let x1 = x0 + step;
            let z0 = -half + iz as f32 * step;
            let z1 = z0 + step;
            let odd = (ix + iz) % 2 == 1;
            let col = if odd { light } else { dark };
            // floor (upward)
            push_quad(
                [x0, 0.0, z0],
                [x1, 0.0, z0],
                [x1, 0.0, z1],
                [x0, 0.0, z1],
                col,
            );
            // ceiling (downward) — flip checker so seams match walls
            let col_c = if odd { dark } else { light };
            push_quad(
                [x0, height, z0],
                [x0, height, z1],
                [x1, height, z1],
                [x1, height, z0],
                col_c,
            );
        }
    }

    // Walls: +Z, -Z, +X, -X
    for i in 0..cells {
        for j in 0..cells {
            let a0 = -half + i as f32 * step;
            let a1 = a0 + step;
            let y0 = j as f32 * (height / cells as f32);
            let y1 = y0 + height / cells as f32;
            let odd = (i + j) % 2 == 1;
            let col = if odd { light } else { dark };
            // +Z wall (z = +half), facing -Z
            push_quad(
                [a0, y0, half],
                [a1, y0, half],
                [a1, y1, half],
                [a0, y1, half],
                col,
            );
            // -Z wall
            push_quad(
                [a1, y0, -half],
                [a0, y0, -half],
                [a0, y1, -half],
                [a1, y1, -half],
                col,
            );
            // +X wall
            push_quad(
                [half, y0, a1],
                [half, y0, a0],
                [half, y1, a0],
                [half, y1, a1],
                col,
            );
            // -X wall
            push_quad(
                [-half, y0, a0],
                [-half, y0, a1],
                [-half, y1, a1],
                [-half, y1, a0],
                col,
            );
        }
    }
    v
}

/// Unit cube as triangle list, colour per-vertex.
#[allow(dead_code)]
fn box_mesh(col: [f32; 3]) -> Vec<[f32; 6]> {
    let (r, g, b) = (col[0], col[1], col[2]);
    let p = [
        [-0.5f32, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];
    let faces: [[usize; 4]; 6] = [
        [0, 1, 2, 3],
        [4, 5, 6, 7],
        [0, 4, 7, 3],
        [1, 5, 6, 2],
        [3, 2, 6, 7],
        [0, 1, 5, 4],
    ];
    let mut out = Vec::new();
    for f in faces {
        let (a, b0, c0, d) = (p[f[0]], p[f[1]], p[f[2]], p[f[3]]);
        for tri in [[a, b0, c0], [a, c0, d]] {
            for pt in tri {
                out.push([pt[0], pt[1], pt[2], r, g, b]);
            }
        }
    }
    out
}

struct ControllerUi {
    tp_x: f32,
    tp_y: f32,
    tp_touch: bool,
    tp_click: bool,
    /// Short labels of other pressed digital inputs (menu, vol, …).
    buttons: Vec<&'static str>,
}

fn controller_ui(inputs: &std::collections::HashMap<String, InputSample>) -> ControllerUi {
    let mut ui = ControllerUi {
        tp_x: 0.0,
        tp_y: 0.0,
        tp_touch: false,
        tp_click: false,
        buttons: Vec::new(),
    };
    for (id, s) in inputs {
        if id.contains("/tp/val") {
            ui.tp_x = s.axis.unwrap_or(0.0);
            ui.tp_y = -s.axis_y.unwrap_or(0.0); // protocol Y is inverted vs OpenGL pad
        } else if id.contains("/tp/touch") {
            ui.tp_touch = s.pressed;
        } else if id.contains("/tp/click") {
            ui.tp_click = s.pressed;
        } else if s.pressed {
            let label: &'static str = if id.contains("/menu") {
                "menu"
            } else if id.contains("vol/+") {
                "vol+"
            } else if id.contains("vol/-") {
                "vol-"
            } else if id.contains("/sys") {
                "sys"
            } else if id.contains("/app") {
                "app"
            } else if id.contains("/tr") {
                "trig"
            } else {
                "btn"
            };
            if !ui.buttons.contains(&label) {
                ui.buttons.push(label);
            }
        }
    }
    ui
}

/// Run the OpenGL visualizer on the **calling** thread (must be main).
/// Blocks until the window is closed.
pub fn run_window(
    poses: Arc<Mutex<LatestPoses>>,
    live_video: Option<LiveVideoSlot>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("OpenGL pose visualizer starting on main thread (RELIVEVR_VIZ)");
    use glutin::config::ConfigTemplateBuilder;
    use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
    use glutin::display::GetGlDisplay;
    use glutin::prelude::*;
    use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
    use glutin_winit::{DisplayBuilder, GlWindow};
    use raw_window_handle::HasRawWindowHandle;
    use winit::dpi::LogicalSize;
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::WindowBuilder;

    // Prefer X11 when both backends exist (common on NixOS / hybrid sessions).
    if std::env::var_os("WINIT_UNIX_BACKEND").is_none() {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
        info!("WINIT_UNIX_BACKEND defaulted to x11");
    }
    info!(
        "DISPLAY={:?} WAYLAND_DISPLAY={:?} WINIT_UNIX_BACKEND={:?}",
        std::env::var_os("DISPLAY"),
        std::env::var_os("WAYLAND_DISPLAY"),
        std::env::var_os("WINIT_UNIX_BACKEND"),
    );

    // Must be main thread — caller starts tokio on a background thread.
    let event_loop = EventLoop::new().map_err(|e| format!("EventLoop::new failed: {e:?}"))?;
    let window_builder = WindowBuilder::new()
        .with_title("ReliveVR pose visualizer")
        .with_inner_size(LogicalSize::new(960.0, 720.0));

    // Minimal GL config — avoid alpha/sample requirements that some drivers reject.
    let template = ConfigTemplateBuilder::new();
    let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));
    let (window, gl_config) = display_builder
        .build(&event_loop, template, |configs| {
            configs
                .reduce(|a, b| {
                    if a.num_samples() > b.num_samples() {
                        a
                    } else {
                        b
                    }
                })
                .expect("no GL configs offered by the display")
        })
        .map_err(|e| format!("glutin DisplayBuilder::build failed: {e:?}"))?;
    let window = window.ok_or("DisplayBuilder produced no window")?;

    let raw = window.raw_window_handle();
    let gl_display = gl_config.display();

    // Try core 3.3 → 3.0 → GLES 3.0.
    let not_current = {
        let attempts = [
            ContextApi::OpenGl(Some(Version::new(3, 3))),
            ContextApi::OpenGl(Some(Version::new(3, 0))),
            ContextApi::Gles(Some(Version::new(3, 0))),
            ContextApi::OpenGl(None),
        ];
        let mut last_err = None;
        let mut created = None;
        for api in attempts {
            let attrs = ContextAttributesBuilder::new()
                .with_context_api(api)
                .build(Some(raw));
            match unsafe { gl_display.create_context(&gl_config, &attrs) } {
                Ok(ctx) => {
                    info!("GL context created with api={api:?}");
                    created = Some(ctx);
                    break;
                }
                Err(e) => {
                    info!("GL context api={api:?} failed: {e:?}");
                    last_err = Some(e);
                }
            }
        }
        created.ok_or_else(|| {
            format!(
                "create_context failed for all APIs; last error: {:?}",
                last_err
            )
        })?
    };

    let attrs = window.build_surface_attributes(SurfaceAttributesBuilder::<WindowSurface>::new());
    let surface = unsafe {
        gl_display
            .create_window_surface(&gl_config, &attrs)
            .map_err(|e| format!("create_window_surface failed: {e:?}"))?
    };
    let context = not_current
        .make_current(&surface)
        .map_err(|e| format!("make_current failed: {e:?}"))?;

    let gl = unsafe {
        glow::Context::from_loader_function_cstr(|s| {
            gl_display.get_proc_address(s) as *const _
        })
    };

    let program = unsafe { compile_program(&gl)? };
    let (grid, axes, hmd_box, ctrl_box) = unsafe {
        (
            Mesh::triangles(&gl, &room_mesh()),
            Mesh::lines(&gl, &axis_mesh()),
            Mesh::lines(&gl, &box_edges([0.3, 0.7, 1.0])),
            Mesh::lines(&gl, &box_edges([1.0, 0.6, 0.2])),
        )
    };
    let u_mvp = unsafe { gl.get_uniform_location(program, "u_mvp") };

    unsafe {
        gl.enable(glow::DEPTH_TEST);
        gl.clear_color(0.08, 0.09, 0.11, 1.0);
    }

    // Offscreen target for H.264 encode (fixed size).
    let (enc_w, enc_h) = encode_dims();
    let (enc_fbo, encode_tx) =
        if live_video.is_some() {
            let (fbo, tex, rb) = unsafe { create_encode_fbo(&gl, enc_w as i32, enc_h as i32)? };
            // ONE encoder for both eyes so SPS/PPS match VideoInit (dual ffmpeg
            // processes produced different param sets → MediaCodec solid green).
            let mut enc = H264Encoder::new(enc_w, enc_h)
                .map_err(|e| format!("H264Encoder: {e}"))?;
            let nbytes = (enc_w * enc_h * 4) as usize;
            // FFmpeg already has warm-up SPS; OpenH264 needs one seed frame.
            if enc.param_sets().is_empty() {
                let black = vec![0u8; nbytes];
                match enc.encode_rgba(&black, true) {
                    Ok((nals, _)) => {
                        tracing::info!("seed encode AU {}B", nals.len());
                    }
                    Err(e) => tracing::warn!("seed encode: {e}"),
                }
            }
            if let Some(ref slot) = live_video {
                let ps = enc.param_sets();
                if !ps.is_empty() {
                    encode::publish_param_sets(slot, ps);
                } else {
                    tracing::warn!("no SPS/PPS after encoder init — VideoInit will defer");
                }
            }
            let fps = target_encode_fps();
            info!(
                "Live stereo encode FBO {enc_w}x{enc_h} target {fps:.0} fps (single encoder; override RELIVEVR_ENCODE_W/H)"
            );
            let (encode_tx, encode_rx) = std::sync::mpsc::sync_channel::<(Vec<u8>, Vec<u8>)>(1);
            let slot_worker = live_video.clone();
            std::thread::Builder::new()
                .name("relivevr-encode".into())
                .spawn(move || {
                    let mut enc = enc;
                    while let Ok((rgba_l, rgba_r)) = encode_rx.recv() {
                        let Some(ref slot) = slot_worker else { continue };
                        // Sequential: same encoder → identical SPS/PPS for both eyes.
                        let left = match enc.encode_rgba(&rgba_l, true) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::warn!("encode left: {e}");
                                continue;
                            }
                        };
                        let right = match enc.encode_rgba(&rgba_r, true) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::warn!("encode right: {e}");
                                continue;
                            }
                        };
                        let (ln, lidr) = left;
                        let (rn, ridr) = right;
                        // Skip near-empty AUs (skip/SEI-only) — they decode as green.
                        let min_au = 200usize;
                        if ln.len() < min_au || rn.len() < min_au {
                            tracing::debug!(
                                "skip tiny AU L={}B R={}B",
                                ln.len(),
                                rn.len()
                            );
                            continue;
                        }
                        encode::publish_stereo(
                            slot,
                            ln,
                            rn,
                            lidr || ridr,
                            enc.pts_us(),
                            enc.frame_index(),
                        );
                    }
                })
                .map_err(|e| format!("encode worker: {e}"))?;
            let _keep = (tex, rb);
            let _ = nbytes;
            (Some(fbo), Some(encode_tx))
        } else {
            (None, None)
        };
    let mut rgba_left = vec![0u8; (enc_w as usize) * (enc_h as usize) * 4];
    let mut rgba_right = vec![0u8; rgba_left.len()];
    let mut encode_every = 0u64;
    let mut last_encode = Instant::now();
    // Vertical FOV in degrees. HelloResponse advertises ~100° H/V;
    // measured Daydream View ~89° total; start at 90° (vol+/- still 40–120).
    let mut fov_deg: f32 = 90.0;
    let mut vol_plus_was = false;
    let mut vol_minus_was = false;

    let mut size = window.inner_size();
    let mut last_title = Instant::now();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(s) => {
                    size = s;
                    if s.width > 0 && s.height > 0 {
                        surface.resize(
                            &context,
                            s.width.try_into().unwrap(),
                            s.height.try_into().unwrap(),
                        );
                        unsafe {
                            gl.viewport(0, 0, s.width as i32, s.height as i32);
                        }
                    }
                }
                WindowEvent::RedrawRequested => {
                    let snap = poses.lock().ok().map(|p| p.clone());
                    let aspect = (size.width as f32).max(1.0) / (size.height as f32).max(1.0);
                    // Vol+/- edge: adjust FOV (logged in title)
                    if let Some(ref state) = snap {
                        let ui = controller_ui(&state.inputs);
                        let vp = ui.buttons.iter().any(|b| *b == "vol+");
                        let vm = ui.buttons.iter().any(|b| *b == "vol-");
                        if vp && !vol_plus_was {
                            fov_deg = (fov_deg + 5.0).min(120.0);
                            info!("FOV → {fov_deg:.0}° (vol+)");
                        }
                        if vm && !vol_minus_was {
                            fov_deg = (fov_deg - 5.0).max(40.0);
                            info!("FOV → {fov_deg:.0}° (vol-)");
                        }
                        vol_plus_was = vp;
                        vol_minus_was = vm;
                    }
                    let proj = Mat4::perspective(fov_deg.to_radians(), aspect, 0.08, 40.0);
                    let view = snap
                        .as_ref()
                        .and_then(|s| s.hmd.as_ref())
                        .map(|h| view_from_hmd(h, 0.0, 0.064))
                        .unwrap_or_else(default_orbit_view);
                    let vp = proj.mul(view);

                    unsafe {
                        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
                        gl.use_program(Some(program));

                        set_mvp(&gl, u_mvp.as_ref(), vp);
                        grid.draw_tris(&gl);
                        axes.draw_lines(&gl);

                        if let Some(ref state) = snap {
                            if let Some(h) = &state.hmd {
                                draw_tracked(
                                    &gl,
                                    u_mvp.as_ref(),
                                    vp,
                                    h,
                                    &hmd_box,
                                    [0.12, 0.08, 0.06], // HMD size metres-ish
                                );
                            }
                            if let Some(c) = &state.ctrl_right {
                                draw_tracked(
                                    &gl,
                                    u_mvp.as_ref(),
                                    vp,
                                    c,
                                    &ctrl_box,
                                    [0.05, 0.04, 0.12],
                                );
                            }
                            // Trackpad + digital buttons (menu, vol, …) on the controller
                            let ui = controller_ui(&state.inputs);
                            if let Some(c) = &state.ctrl_right {
                                draw_controller_gizmo(
                                    &gl,
                                    program,
                                    u_mvp.as_ref(),
                                    vp,
                                    c,
                                    &ui,
                                );
                            }
                        }

                        // Stereo FBO → dual H.264 → shared slot
                        // Encode path: GL readback on this thread, CPU encode on worker.
                        // try_send drops the frame if the worker is still busy — desktop
                        // redraw is never blocked on ffmpeg.
                        if let (Some(fbo), Some(tx)) = (enc_fbo, encode_tx.as_ref()) {
                            encode_every = encode_every.wrapping_add(1);
                            let min_dt = Duration::from_secs_f32(1.0 / target_encode_fps().max(1.0));
                            if last_encode.elapsed() >= min_dt {
                                last_encode = Instant::now();
                                unsafe {
                                    prepare_stereo_rgba(
                                        &gl,
                                        fbo,
                                        &mut rgba_left,
                                        &mut rgba_right,
                                        program,
                                        u_mvp.as_ref(),
                                        &grid,
                                        &axes,
                                        &hmd_box,
                                        &ctrl_box,
                                        &snap,
                                        fov_deg,
                                        enc_w,
                                        enc_h,
                                    );
                                }
                                match tx.try_send((rgba_left.clone(), rgba_right.clone())) {
                                    Ok(()) => {}
                                    Err(std::sync::mpsc::TrySendError::Full(_)) => {
                                        // Worker busy — drop this encode frame.
                                        if encode_every < 5 || encode_every % 300 == 0 {
                                            tracing::debug!("encode worker busy; dropping frame");
                                        }
                                    }
                                    Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                                        tracing::warn!("encode worker disconnected");
                                    }
                                }
                            }
                        }
                    }

                    if last_title.elapsed() > Duration::from_millis(200) {
                        last_title = Instant::now();
                        let title = match &snap {
                            Some(s) => format!(
                                "ReliveVR viz — FOV={:.0}°  updates={}  {}",
                                fov_deg,
                                s.updates,
                                s.summary_line()
                            ),
                            None => "ReliveVR viz".into(),
                        };
                        window.set_title(&title);
                    }

                    let _ = surface.swap_buffers(&context);
                }
                _ => {}
            },
            Event::AboutToWait => {
                window.request_redraw();
                std::thread::sleep(Duration::from_millis(8)); // ~120 Hz cap
            }
            _ => {}
        }
    })?;
    Ok(())
}

fn box_edges(col: [f32; 3]) -> Vec<[f32; 6]> {
    let (r, g, b) = (col[0], col[1], col[2]);
    let p = [
        [-0.5f32, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    let mut v = Vec::new();
    for (a, b0) in edges {
        v.push([p[a][0], p[a][1], p[a][2], r, g, b]);
        v.push([p[b0][0], p[b0][1], p[b0][2], r, g, b]);
    }
    // Local axes from center
    v.push([0.0, 0.0, 0.0, 1.0, 0.3, 0.3]);
    v.push([0.75, 0.0, 0.0, 1.0, 0.3, 0.3]);
    v.push([0.0, 0.0, 0.0, 0.3, 1.0, 0.3]);
    v.push([0.0, 0.75, 0.0, 0.3, 1.0, 0.3]);
    v.push([0.0, 0.0, 0.0, 0.3, 0.5, 1.0]);
    v.push([0.0, 0.0, 0.75, 0.3, 0.5, 1.0]);
    v
}

unsafe fn draw_tracked(
    gl: &glow::Context,
    u_mvp: Option<&glow::UniformLocation>,
    vp: Mat4,
    pose: &TrackedPose,
    mesh: &Mesh,
    scale: [f32; 3],
) {
    let model = Mat4::from_quat_pos(pose.orient, pose.pos).mul(Mat4::scale(scale));
    set_mvp(gl, u_mvp, vp.mul(model));
    mesh.draw_lines(gl);
}

unsafe fn draw_controller_gizmo(
    gl: &glow::Context,
    program: glow::Program,
    u_mvp: Option<&glow::UniformLocation>,
    vp: Mat4,
    pose: &TrackedPose,
    ui: &ControllerUi,
) {
    // Local-space line mesh on the controller: trackpad + button lamps.
    let col = if ui.tp_click {
        [1.0, 0.2, 0.2]
    } else if ui.tp_touch {
        [0.2, 1.0, 0.4]
    } else {
        [0.5, 0.5, 0.55]
    };
    let pad = 0.4f32;
    let mut verts = vec![
        [-pad, 0.55, -pad, col[0], col[1], col[2]],
        [pad, 0.55, -pad, col[0], col[1], col[2]],
        [pad, 0.55, -pad, col[0], col[1], col[2]],
        [pad, 0.55, pad, col[0], col[1], col[2]],
        [pad, 0.55, pad, col[0], col[1], col[2]],
        [-pad, 0.55, pad, col[0], col[1], col[2]],
        [-pad, 0.55, pad, col[0], col[1], col[2]],
        [-pad, 0.55, -pad, col[0], col[1], col[2]],
    ];
    let fx = ui.tp_x * pad;
    let fz = ui.tp_y * pad;
    let ic = if ui.tp_touch {
        [1.0, 1.0, 0.2]
    } else {
        [0.6, 0.6, 0.2]
    };
    verts.push([fx - 0.05, 0.56, fz, ic[0], ic[1], ic[2]]);
    verts.push([fx + 0.05, 0.56, fz, ic[0], ic[1], ic[2]]);
    verts.push([fx, 0.56, fz - 0.05, ic[0], ic[1], ic[2]]);
    verts.push([fx, 0.56, fz + 0.05, ic[0], ic[1], ic[2]]);

    // Digital buttons as small squares above the pad (magenta when held).
    // Layout: menu | vol+ | vol- | sys | app | trig | btn
    let slots: &[(&str, f32)] = &[
        ("menu", -0.35),
        ("vol+", -0.20),
        ("vol-", -0.05),
        ("sys", 0.10),
        ("app", 0.25),
        ("trig", 0.35),
        ("btn", 0.45),
    ];
    let by = 0.75f32;
    let bs = 0.06f32;
    for (name, bx) in slots {
        let on = ui.buttons.iter().any(|b| b == name);
        let c = if on {
            [1.0, 0.2, 1.0] // magenta = pressed
        } else {
            [0.25, 0.25, 0.3]
        };
        verts.push([bx - bs, by, -bs, c[0], c[1], c[2]]);
        verts.push([bx + bs, by, -bs, c[0], c[1], c[2]]);
        verts.push([bx + bs, by, -bs, c[0], c[1], c[2]]);
        verts.push([bx + bs, by, bs, c[0], c[1], c[2]]);
        verts.push([bx + bs, by, bs, c[0], c[1], c[2]]);
        verts.push([bx - bs, by, bs, c[0], c[1], c[2]]);
        verts.push([bx - bs, by, bs, c[0], c[1], c[2]]);
        verts.push([bx - bs, by, -bs, c[0], c[1], c[2]]);
    }

    let mesh = Mesh::lines(gl, &verts);
    let model = Mat4::from_quat_pos(pose.orient, pose.pos).mul(Mat4::scale([0.12, 0.12, 0.12]));
    gl.use_program(Some(program));
    set_mvp(gl, u_mvp, vp.mul(model));
    mesh.draw_lines(gl);
    let _ = (mesh.vao, mesh.vbo);
}

unsafe fn set_mvp(gl: &glow::Context, loc: Option<&glow::UniformLocation>, m: Mat4) {
    if let Some(l) = loc {
        gl.uniform_matrix_4_f32_slice(Some(l), false, &m.0);
    }
}


unsafe fn create_encode_fbo(
    gl: &glow::Context,
    w: i32,
    h: i32,
) -> Result<(glow::Framebuffer, glow::Texture, glow::Renderbuffer), String> {
    let tex = gl.create_texture().map_err(|e| e.to_string())?;
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA as i32,
        w,
        h,
        0,
        glow::RGBA,
        glow::UNSIGNED_BYTE,
        None,
    );
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);

    let rb = gl.create_renderbuffer().map_err(|e| e.to_string())?;
    gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rb));
    gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH_COMPONENT24, w, h);

    let fbo = gl.create_framebuffer().map_err(|e| e.to_string())?;
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
    gl.framebuffer_texture_2d(
        glow::FRAMEBUFFER,
        glow::COLOR_ATTACHMENT0,
        glow::TEXTURE_2D,
        Some(tex),
        0,
    );
    gl.framebuffer_renderbuffer(
        glow::FRAMEBUFFER,
        glow::DEPTH_ATTACHMENT,
        glow::RENDERBUFFER,
        Some(rb),
    );
    let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
    gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    if status != glow::FRAMEBUFFER_COMPLETE {
        return Err(format!("FBO incomplete: {status:#x}"));
    }
    Ok((fbo, tex, rb))
}

const IPD_M: f32 = 0.064;

unsafe fn render_eye(
    gl: &glow::Context,
    fbo: glow::Framebuffer,
    program: glow::Program,
    u_mvp: Option<&glow::UniformLocation>,
    grid: &Mesh,
    axes: &Mesh,
    _hmd_box: &Mesh,
    ctrl_box: &Mesh,
    snap: &Option<LatestPoses>,
    view: Mat4,
    rgba: &mut [u8],
    fov_deg: f32,
    enc_w: u32,
    enc_h: u32,
) {
    let w = enc_w as i32;
    let h = enc_h as i32;
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
    gl.viewport(0, 0, w, h);
    gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
    gl.use_program(Some(program));

    let aspect = enc_w as f32 / enc_h as f32;
    let proj = Mat4::perspective(fov_deg.to_radians(), aspect, 0.08, 40.0);
    let vp = proj.mul(view);
    set_mvp(gl, u_mvp, vp);
    grid.draw_tris(gl);
    axes.draw_lines(gl);
    if let Some(state) = snap {
        // Skip HMD box at the camera.
        if let Some(c) = &state.ctrl_right {
            draw_tracked(gl, u_mvp, vp, c, ctrl_box, [0.05, 0.04, 0.12]);
            let ui = controller_ui(&state.inputs);
            draw_controller_gizmo(gl, program, u_mvp, vp, c, &ui);
        }
    }

    gl.read_pixels(
        0,
        0,
        w,
        h,
        glow::RGBA,
        glow::UNSIGNED_BYTE,
        glow::PixelPackData::Slice(rgba),
    );
    gl.bind_framebuffer(glow::FRAMEBUFFER, None);
}

/// Render both eyes into `rgba_left` / `rgba_right` (GL thread only).
unsafe fn prepare_stereo_rgba(
    gl: &glow::Context,
    fbo: glow::Framebuffer,
    rgba_left: &mut [u8],
    rgba_right: &mut [u8],
    program: glow::Program,
    u_mvp: Option<&glow::UniformLocation>,
    grid: &Mesh,
    axes: &Mesh,
    hmd_box: &Mesh,
    ctrl_box: &Mesh,
    snap: &Option<LatestPoses>,
    fov_deg: f32,
    enc_w: u32,
    enc_h: u32,
) {
    let (view_l, view_r) = match snap.as_ref().and_then(|s| s.hmd.as_ref()) {
        Some(h) => (
            view_from_hmd(h, -1.0, IPD_M),
            view_from_hmd(h, 1.0, IPD_M),
        ),
        None => (default_orbit_view(), default_orbit_view()),
    };
    render_eye(gl, fbo, program, u_mvp, grid, axes, hmd_box, ctrl_box, snap, view_l, rgba_left, fov_deg, enc_w, enc_h);
    render_eye(gl, fbo, program, u_mvp, grid, axes, hmd_box, ctrl_box, snap, view_r, rgba_right, fov_deg, enc_w, enc_h);
}

unsafe fn compile_program(gl: &glow::Context) -> Result<glow::Program, String> {
    let vs = gl.create_shader(glow::VERTEX_SHADER).map_err(|e| e.to_string())?;
    gl.shader_source(vs, VS);
    gl.compile_shader(vs);
    if !gl.get_shader_compile_status(vs) {
        return Err(gl.get_shader_info_log(vs));
    }
    let fs = gl.create_shader(glow::FRAGMENT_SHADER).map_err(|e| e.to_string())?;
    gl.shader_source(fs, FS);
    gl.compile_shader(fs);
    if !gl.get_shader_compile_status(fs) {
        return Err(gl.get_shader_info_log(fs));
    }
    let prog = gl.create_program().map_err(|e| e.to_string())?;
    gl.attach_shader(prog, vs);
    gl.attach_shader(prog, fs);
    gl.link_program(prog);
    if !gl.get_program_link_status(prog) {
        return Err(gl.get_program_info_log(prog));
    }
    gl.delete_shader(vs);
    gl.delete_shader(fs);
    Ok(prog)
}
