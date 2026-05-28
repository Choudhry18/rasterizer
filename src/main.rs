use std::num::NonZeroU32;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use std::rc::Rc;

struct RasterizerApp {
    window: Option<Rc<Window>>,
    context: Option<softbuffer::Context<Rc<Window>>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
}

impl ApplicationHandler for RasterizerApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Rust Software Rasterizer (Winit)");
        
        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());
        
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);
    }

    // 2. Handles OS events (clicks, keypresses, resizing, closing)
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

                // Resize the buffer surface to match the window
                surface.resize(
                    NonZeroU32::new(size.width).unwrap(),
                    NonZeroU32::new(size.height).unwrap(),
                ).unwrap();

                // Get a mutable reference to the screen pixels
                let mut buffer = surface.buffer_mut().unwrap();

                for pixel in buffer.iter_mut() {
                    *pixel = 0xFF_1A_1A_2E; // Format is 0xAA_RR_GG_BB
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
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll); 

    let mut app = RasterizerApp {
        window: None,
        context: None,
        surface: None,
    };

    event_loop.run_app(&mut app).unwrap();
}