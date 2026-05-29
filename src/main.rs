use std::num::NonZeroU32;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use std::rc::Rc;
use io::Mesh;

use crate::io::load_mesh_as_ndarray;
mod io;

struct RasterizerApp {
    window: Option<Rc<Window>>,
    context: Option<softbuffer::Context<Rc<Window>>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    mesh: Mesh,
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

                for triangle in self.mesh.triangles.iter(){

                    let v0 = &triangle.v0;
                    let v1 = &triangle.v1;
                    let v2 = &triangle.v2;

                    if v0[2] <= 0.0 || v1[2] <= 0.0 || v2[2] <= 0.0 {continue; }

                    let p0_proj_x = v0[0] / v0[2];
                    let p0_proj_y = v0[1] / v0[2];

                    let p1_proj_x = v1[0] / v1[2];
                    let p1_proj_y = v1[1] / v1[2];

                    let p2_proj_x = v2[0] / v2[2];
                    let p2_proj_y = v2[1] / v2[2];

                    let pixel0_x = ((p0_proj_x + 1.0) / 2.0) * width;
                    let pixel0_y = ((1.0 - p0_proj_y) / 2.0) * height;

                    let pixel1_x = ((p1_proj_x + 1.0) / 2.0) * width;
                    let pixel1_y = ((1.0 - p1_proj_y) / 2.0) * height;

                    let pixel2_x = ((p2_proj_x + 1.0) / 2.0) * width;
                    let pixel2_y = ((1.0 - p2_proj_y) / 2.0) * height;

                    let idx0 = (pixel0_y as usize * width as usize) + pixel0_x as usize;
                    let idx1 = (pixel1_y as usize * width as usize) + pixel1_x as usize;
                    let idx2 = (pixel2_y as usize * width as usize) + pixel2_x as usize;

                    if idx0 < buffer.len() { buffer[idx0] = 0xFF_FF_FF_FF; } // Draw a white dot
                    if idx1 < buffer.len() { buffer[idx1] = 0xFF_FF_FF_FF; }
                    if idx2 < buffer.len() { buffer[idx2] = 0xFF_FF_FF_FF; }
                    
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
    };

    event_loop.run_app(&mut app).unwrap();
}