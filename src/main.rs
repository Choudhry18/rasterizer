use std::num::NonZeroU32;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use winit::event::ElementState;
use winit::keyboard::{Key, ModifiersState};
use winit::event::MouseScrollDelta;
use std::rc::Rc;
use rasterizer::camera;
use rasterizer::io::{Mesh, load_obj};
use rasterizer::render::{self, FrameOptions};


struct RasterizerApp {
    window: Option<Rc<Window>>,
    context: Option<softbuffer::Context<Rc<Window>>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    models: Vec<Model>,
    depth: Vec<f32>,
    camera_state: camera::CameraState,
    mouse_down: bool,
    last_cursor: Option<(f64, f64)>,
    modifiers: ModifiersState,
    fov_y: f32,
}

struct Model {
    mesh: Mesh,
    /// Model matrix (model → world). The general form combines rotation `R`,
    /// scale `S`, and translation `t`:
    ///
    /// ```text
    /// M = [ R·S   t ]
    ///     [  0    1 ]
    /// ```
    ///
    /// The instance built in `main` is a pure 45° rotation about the Y axis
    /// (`cos 45° ≈ sin 45° ≈ 0.707`, `S = I`, `t = 0`):
    ///
    /// ```text
    /// R_y(θ) = [  cos θ   0   sin θ   0 ]
    ///          [    0     1     0     0 ]
    ///          [ −sin θ   0   cos θ   0 ]
    ///          [    0     0     0     1 ]
    /// ```
    transform: [[f32;4];4],
}

const PAN_SENS: f32 = 0.01;
const ORBIT_SENS: f32 = 0.005;
const DOLLY_SENS: f32 = 0.5;
const FOV_Y: f32 = std::f32::consts::FRAC_PI_3;  // 60° — starting field of view
const FOV_STEP: f32 = std::f32::consts::PI / 180.0;  // 1° per +/- keypress
const FOV_MIN: f32 = std::f32::consts::PI / 18.0;    // 10°
const FOV_MAX: f32 = std::f32::consts::PI * 17.0 / 18.0;  // 170°

impl ApplicationHandler for RasterizerApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Rasterizer");

        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
             WindowEvent::ModifiersChanged(new) => {
                self.modifiers = new.state();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if let Key::Character(c) = &event.logical_key {
                        match c.as_str() {
                            // "=" so widening doesn't require holding Shift for "+"
                            "+" | "=" => self.fov_y = (self.fov_y + FOV_STEP).min(FOV_MAX),
                            "-" | "_" => self.fov_y = (self.fov_y - FOV_STEP).max(FOV_MIN),
                            _ => {}
                        }
                    }
                }
            }

            WindowEvent::MouseInput {state, ..}=> {
                self.mouse_down = state == ElementState::Pressed;
                if !self.mouse_down { self.last_cursor = None; }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p)   => (p.y as f32) * 0.01,
                };
                self.camera_state.dolly(amount * DOLLY_SENS);
            }

            WindowEvent::CursorMoved {position, .. } => {

                let pos = (position.x, position.y);
                if self.mouse_down {
                    if let Some((lx, ly)) = self.last_cursor {
                        let dx = (pos.0 - lx) as f32;
                        let dy = (pos.1 - ly) as f32;

                        if self.modifiers.control_key(){
                            self.camera_state.orbit(-dx * ORBIT_SENS, -dy * ORBIT_SENS);
                        }else{
                            self.camera_state.pan(-dx * PAN_SENS, dy * PAN_SENS);
                        }
                    }
                }
                self.last_cursor = Some(pos);
            }

            WindowEvent::RedrawRequested => {
                let (Some(surface), Some(window)) = (&mut self.surface, &self.window) else { return; };
                let size = window.inner_size();

                if size.width == 0 || size.height == 0 { return; }

                surface.resize(
                    NonZeroU32::new(size.width).unwrap(),
                    NonZeroU32::new(size.height).unwrap(),
                ).unwrap();

                let mut buffer = surface.buffer_mut().unwrap();

                let width = size.width as usize;
                let height = size.height as usize;
                self.depth.resize(width * height, f32::INFINITY);

                let opts = FrameOptions { fov_y: self.fov_y, ..FrameOptions::default() };

                // render_frame clears the buffers, so this viewer keeps
                // single-model semantics (only one Model is ever constructed).
                if let Some(model) = self.models.first() {
                    render::render_frame(
                        &model.mesh, &model.transform, &self.camera_state, &opts,
                        width, height, &mut buffer, &mut self.depth,
                    );
                }

                buffer.present().unwrap();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Error: Missing mesh file path.");
        eprintln!("Usage: cargo run -- <path_to_obj_file>");
        std::process::exit(1);

    }

    let mesh_path = &args[1];

    let mesh = load_obj(mesh_path);

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = RasterizerApp {
        window: None,
        context: None,
        surface: None,
        models: vec![Model{
            mesh,
            transform : [[0.707,  0.0, 0.707, 0.0],
                         [0.0,    1.0, 0.0,   0.0],
                         [-0.707, 0.0, 0.707, 0.0],
                         [0.0,    0.0, 0.0,   1.0]],
                        }
                    ],
        depth: Vec::new(),
        camera_state: camera::CameraState::new(
            [0.0, 0.0, 0.0],         // cam_to (target)
            [0.0, 1.0, 0.0],         // cam_up (world up)
            std::f32::consts::PI,    // yaw — start camera on -Z (behind target)
            0.0,                     // pitch — horizon level
            8.0,                     // radius — 8 units from target
        ),
        mouse_down: false,
        last_cursor: None,
        modifiers: ModifiersState::empty(),
        fov_y: FOV_Y,

    };

    event_loop.run_app(&mut app).unwrap();
}
