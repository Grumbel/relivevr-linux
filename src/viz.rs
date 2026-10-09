//! Optional OpenGL window visualizing HMD + controller poses from `LatestPoses`.
//!
//! Enable with `RELIVEVR_VIZ=1`. The event loop must run on the **main** thread
//! (winit requirement). The UDP server then runs on a background tokio runtime.
//! Later this path will render to an FBO for realtime encode.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use glow::HasContext;
use tracing::info;

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

struct Mesh {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    count: i32,
}

impl Mesh {
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

fn grid_mesh() -> Vec<[f32; 6]> {
    let mut v = Vec::new();
    let n = 10;
    let step = 0.25f32;
    let c = [0.25f32, 0.25, 0.28];
    for i in -n..=n {
        let t = i as f32 * step;
        v.push([-n as f32 * step, 0.0, t, c[0], c[1], c[2]]);
        v.push([n as f32 * step, 0.0, t, c[0], c[1], c[2]]);
        v.push([t, 0.0, -n as f32 * step, c[0], c[1], c[2]]);
        v.push([t, 0.0, n as f32 * step, c[0], c[1], c[2]]);
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

fn trackpad_state(inputs: &std::collections::HashMap<String, InputSample>) -> (f32, f32, bool, bool) {
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let mut touch = false;
    let mut click = false;
    for (id, s) in inputs {
        if id.contains("/tp/val") {
            x = s.axis.unwrap_or(0.0);
            y = s.axis_y.unwrap_or(0.0);
        } else if id.contains("/tp/touch") {
            touch = s.pressed;
        } else if id.contains("/tp/click") {
            click = s.pressed;
        }
    }
    (x, y, touch, click)
}

/// Run the OpenGL visualizer on the **calling** thread (must be main).
/// Blocks until the window is closed.
pub fn run_window(poses: Arc<Mutex<LatestPoses>>) -> Result<(), Box<dyn std::error::Error>> {
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

    // Must be main thread — caller starts tokio on a background thread.
    let event_loop = EventLoop::new()?;
    let window_builder = WindowBuilder::new()
        .with_title("ReliveVR pose visualizer")
        .with_inner_size(LogicalSize::new(960.0, 720.0));

    let template = ConfigTemplateBuilder::new().with_alpha_size(8);
    let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));
    let (window, gl_config) = display_builder.build(&event_loop, template, |configs| {
        configs
            .reduce(|a, b| {
                if a.num_samples() > b.num_samples() {
                    a
                } else {
                    b
                }
            })
            .unwrap()
    })?;
    let window = window.ok_or("no window")?;

    let raw = window.raw_window_handle();
    let gl_display = gl_config.display();
    let context_attrs = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
        .build(Some(raw));
    let not_current = unsafe { gl_display.create_context(&gl_config, &context_attrs)? };

    let attrs = window.build_surface_attributes(SurfaceAttributesBuilder::<WindowSurface>::new());
    let surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs)? };
    let context = not_current.make_current(&surface)?;

    let gl = unsafe {
        glow::Context::from_loader_function_cstr(|s| {
            gl_display.get_proc_address(s) as *const _
        })
    };

    let program = unsafe { compile_program(&gl)? };
    let (grid, axes, hmd_box, ctrl_box) = unsafe {
        (
            Mesh::lines(&gl, &grid_mesh()),
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
                    let proj = Mat4::perspective(50.0f32.to_radians(), aspect, 0.05, 20.0);
                    // Orbit camera looking at origin / average pose
                    let eye = [1.6f32, 1.4, 1.8];
                    let view = Mat4::look_at(eye, [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]);
                    let vp = proj.mul(view);

                    unsafe {
                        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
                        gl.use_program(Some(program));

                        set_mvp(&gl, u_mvp.as_ref(), vp);
                        grid.draw_lines(&gl);
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
                            // Trackpad HUD in NDC-ish overlay via lines at controller
                            let (tx, ty, touch, click) = trackpad_state(&state.inputs);
                            if let Some(c) = &state.ctrl_right {
                                draw_trackpad_gizmo(
                                    &gl,
                                    program,
                                    u_mvp.as_ref(),
                                    vp,
                                    c,
                                    tx,
                                    ty,
                                    touch,
                                    click,
                                );
                            }
                        }
                    }

                    if last_title.elapsed() > Duration::from_millis(200) {
                        last_title = Instant::now();
                        let title = match &snap {
                            Some(s) => format!(
                                "ReliveVR viz — updates={}  {}",
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

unsafe fn draw_trackpad_gizmo(
    gl: &glow::Context,
    program: glow::Program,
    u_mvp: Option<&glow::UniformLocation>,
    vp: Mat4,
    pose: &TrackedPose,
    tx: f32,
    ty: f32,
    touch: bool,
    click: bool,
) {
    // Build a small line mesh for the pad in local controller space, then
    // upload transiently — keep it simple: reconstruct each frame.
    let col = if click {
        [1.0, 0.2, 0.2]
    } else if touch {
        [0.2, 1.0, 0.4]
    } else {
        [0.5, 0.5, 0.55]
    };
    let pad = 0.4f32;
    let mut verts = vec![
        // pad square
        [-pad, 0.55, -pad, col[0], col[1], col[2]],
        [pad, 0.55, -pad, col[0], col[1], col[2]],
        [pad, 0.55, -pad, col[0], col[1], col[2]],
        [pad, 0.55, pad, col[0], col[1], col[2]],
        [pad, 0.55, pad, col[0], col[1], col[2]],
        [-pad, 0.55, pad, col[0], col[1], col[2]],
        [-pad, 0.55, pad, col[0], col[1], col[2]],
        [-pad, 0.55, -pad, col[0], col[1], col[2]],
    ];
    // finger indicator
    let fx = tx * pad;
    let fz = ty * pad;
    let ic = if touch {
        [1.0, 1.0, 0.2]
    } else {
        [0.6, 0.6, 0.2]
    };
    verts.push([fx - 0.05, 0.56, fz, ic[0], ic[1], ic[2]]);
    verts.push([fx + 0.05, 0.56, fz, ic[0], ic[1], ic[2]]);
    verts.push([fx, 0.56, fz - 0.05, ic[0], ic[1], ic[2]]);
    verts.push([fx, 0.56, fz + 0.05, ic[0], ic[1], ic[2]]);

    let mesh = Mesh::lines(gl, &verts);
    let model = Mat4::from_quat_pos(pose.orient, pose.pos).mul(Mat4::scale([0.12, 0.12, 0.12]));
    gl.use_program(Some(program));
    set_mvp(gl, u_mvp, vp.mul(model));
    mesh.draw_lines(gl);
    // leak GPU objects for now is ok for a probe; delete would need gl context lifecycle
    let _ = (mesh.vao, mesh.vbo);
}

unsafe fn set_mvp(gl: &glow::Context, loc: Option<&glow::UniformLocation>, m: Mat4) {
    if let Some(l) = loc {
        gl.uniform_matrix_4_f32_slice(Some(l), false, &m.0);
    }
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
