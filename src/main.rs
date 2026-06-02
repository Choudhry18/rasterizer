mod io;
mod raster;

use std::num::NonZeroU32;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use std::rc::Rc;
use io::Mesh;
use crate::io::load_mesh_as_ndarray;


struct RasterizerApp {
    window: Option<Rc<Window>>,
    context: Option<softbuffer::Context<Rc<Window>>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    mesh: Mesh,
    depth: Vec<f32>,
}

  fn project(v: [f32;4], width: f32, height: f32) -> [f32; 3] {
      let inv_z = 1.0 / v[2];
      let ndc_x = v[0] * inv_z;
      let ndc_y = v[1] * inv_z;

      let px = (ndc_x + 1.0) * 0.5 * width;
      let py = (1.0 - ndc_y) * 0.5 * height;

      [px, py, v[2]]
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

                for triangle in self.mesh.triangles.iter(){


                    if triangle.v0[2] <= 0.0 || triangle.v1[2] <= 0.0 || triangle.v2[2] <= 0.0 {continue; }

                    let p0 = project(triangle.v0, width, height);
                    let p1 = project(triangle.v1, width, height);
                    let p2 = project(triangle.v2, width, height);

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
    };

    event_loop.run_app(&mut app).unwrap();
}