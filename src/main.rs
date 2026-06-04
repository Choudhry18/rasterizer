mod io;
mod raster;
mod camera;


use std::num::NonZeroU32;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use winit::event::ElementState;
use std::rc::Rc;
use io::Mesh;
use crate::io::load_mesh_as_ndarray;


struct RasterizerApp {
    window: Option<Rc<Window>>,
    context: Option<softbuffer::Context<Rc<Window>>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    mesh: Mesh,
    depth: Vec<f32>,
    camera_state: camera::CameraState,
    mouse_down: bool,
    last_cursor: Option<(f64, f64)>,  
}

  fn project(v: [f32;4], width: f32, height: f32) -> [f32; 3] {
      let inv_z = 1.0 / v[2];
      let ndc_x = v[0] * inv_z;
      let ndc_y = v[1] * inv_z;

      let px = (ndc_x + 1.0) * 0.5 * width;
      let py = (1.0 - ndc_y) * 0.5 * height;

      [px, py, v[2]]
  }

  fn mat_mul(a: &[[f32;4];4],b: [f32;4]) -> [f32;4]{
    let row1 = a[0][0] * b[0] + a[0][1] * b[1] + a[0][2] * b[2] + a[0][3] * b[3];
    let row2 = a[1][0] * b[0] + a[1][1] * b[1] + a[1][2] * b[2] + a[1][3] * b[3];
    let row3 = a[2][0] * b[0] + a[2][1] * b[1] + a[2][2] * b[2] + a[2][3] * b[3];
    let row4 = a[3][0] * b[0] + a[3][1] * b[1] + a[3][2] * b[2] + a[3][3] * b[3];

    [row1, row2, row3, row4]
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

            WindowEvent::MouseInput {state, button, ..}=> {
                self.mouse_down = state == ElementState::Pressed;
                if !self.mouse_down { self.last_cursor = None; }
            }

            WindowEvent::CursorMoved {position, .. } => {

                let pos = (position.x, position.y);
                if self.mouse_down {
                    if let Some((lx, ly)) = self.last_cursor {
                        let dx = (pos.0 - lx) as f32;
                        let dy = (pos.1 - ly) as f32;
                        self.camera_state.pan(dx, dy);  
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

                for triangle in self.mesh.triangles.iter(){

                    let v0 = mat_mul(&view, triangle.v0);
                    let v1 = mat_mul(&view, triangle.v1);
                    let v2 = mat_mul(&view, triangle.v2);

                    if v0[2] <= 0.0 || v1[2] <= 0.0 || v2[2] <= 0.0 {continue; }

                    let p0 = project(v0, width, height);
                    let p1 = project(v1, width, height);
                    let p2 = project(v2, width, height);

                    raster::draw_triangle(
                       &mut buffer, &mut self.depth, size.width as usize, size.height as usize,
                        p0, p1, p2);
                    
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
        mesh: mesh,
        depth: Vec::new(),
        camera_state: camera::CameraState::new(
            [0.0, 0.0, -8.0],
            [0.0, 0.0,  0.0],
            [0.0, 1.0,  0.0],
        ),
        mouse_down: false,
        last_cursor: None,
    };

    event_loop.run_app(&mut app).unwrap();
}