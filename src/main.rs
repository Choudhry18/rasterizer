mod io;
mod raster;
mod camera;
mod calculations;

use std::num::NonZeroU32;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use winit::event::ElementState;
use winit::keyboard::ModifiersState;
use winit::event::MouseScrollDelta;
use std::rc::Rc;
use io::Mesh;
use crate::io::load_mesh_as_ndarray;
use crate::calculations::{cross_product, dot_product, mat_mul, normalize, subtract};


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
}

struct Model {
    mesh: Mesh,
    transform: [[f32;4];4],
}

const PAN_SENS: f32 = 0.01;
const ORBIT_SENS: f32 = 0.005;
const DOLLY_SENS: f32 = 0.5;
const FOV_Y: f32 = std::f32::consts::FRAC_PI_3;  // 60°
const NEAR: f32 = 0.1;
const FAR: f32 = 1000.0;

const LIGHT_DIR: [f32; 3] = [0.3, -0.8, 0.5];  
const AMBIENT:   f32      = 0.2;
const BASE_COLOR: u32     = 0xFF_B6_C1;

fn build_projection(fov_y: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov_y * 0.5).tan();
    let d = far / (far - near);
    [
        [f / aspect, 0.0, 0.0,        0.0],
        [0.0,        f,   0.0,        0.0],
        [0.0,        0.0, d,         -near * d],
        [0.0,        0.0, 1.0,        0.0],
    ]
}

fn to_screen(v_clip: [f32; 4], width: f32, height: f32) -> [f32; 3] {
    let inv_w = 1.0 / v_clip[3];
    let ndc_x = v_clip[0] * inv_w;
    let ndc_y = v_clip[1] * inv_w;
    let ndc_z = v_clip[2] * inv_w;

    let px = (ndc_x + 1.0) * 0.5 * width;
    let py = (1.0 - ndc_y) * 0.5 * height;

    [px, py, ndc_z]
}

  fn shade(color: u32, intensity: f32) -> u32 {
      let r = (((color >> 16) & 0xFF) as f32 * intensity) as u32 & 0xFF;
      let g = (((color >>  8) & 0xFF) as f32 * intensity) as u32 & 0xFF;
      let b = (( color        & 0xFF) as f32 * intensity) as u32 & 0xFF;
      0xFF000000 | (r << 16) | (g << 8) | b
  }

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

                for pixel in buffer.iter_mut() {
                    *pixel = 0xFF_1A_1A_2E; 
                }

                let width = size.width as f32;
                let height = size.height as f32;
                
                let pixel_count = size.width as usize * size.height as usize;
                self.depth.resize(pixel_count, f32::INFINITY);
                self.depth.fill(f32::INFINITY);

                let view = self.camera_state.build_view();
                let proj = build_projection(FOV_Y, width / height, NEAR, FAR);

                let light = normalize(LIGHT_DIR);
                let neg_light = [-light[0], -light[1], -light[2]];


                for model in self.models.iter(){
                    for triangle in model.mesh.triangles.iter(){

                        let w0 = mat_mul(&model.transform, triangle.v0);
                        let w1 = mat_mul(&model.transform, triangle.v1); 
                        let w2 = mat_mul(&model.transform, triangle.v2);

                        let v0_cam = mat_mul(&view, w0);
                        let v1_cam = mat_mul(&view, w1);
                        let v2_cam = mat_mul(&view, w2);

                        if v0_cam[2] <= NEAR || v1_cam[2] <= NEAR || v2_cam[2] <= NEAR { continue; }

                        let v0_clip = mat_mul(&proj, v0_cam);
                        let v1_clip = mat_mul(&proj, v1_cam);
                        let v2_clip = mat_mul(&proj, v2_cam);

                        let p0 = to_screen(v0_clip, width, height);
                        let p1 = to_screen(v1_clip, width, height);
                        let p2 = to_screen(v2_clip, width, height);


                        let normal = normalize(cross_product(subtract(w1[..3].try_into().unwrap(), w0[..3].try_into().unwrap()), subtract(w2[..3].try_into().unwrap(), w0[..3].try_into().unwrap())));

                        let diffuse = dot_product(normal, neg_light).max(0.0);
                        let intensity = AMBIENT + (1.0 - AMBIENT) * diffuse;
                        let color = shade(BASE_COLOR, intensity);

                        raster::draw_triangle(
                        &mut buffer, &mut self.depth, size.width as usize, size.height as usize,
                            p0, p1, p2, color);

                    }
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

    let mesh = load_mesh_as_ndarray(mesh_path);

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll); 

    let mut app = RasterizerApp {
        window: None,
        context: None,
        surface: None,
        models: vec![Model{
            mesh,
            transform : [[0.707 , 0.0, 0.707, 0.0],
                         [0.0, 1.0, 0.0, 0.0],
                         [-0.707, 0.0, 0.707, 0.0],
                         [0.0, 0.0, 0.0, 1.0]],
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
        modifiers: ModifiersState::empty()

    };

    event_loop.run_app(&mut app).unwrap();
}