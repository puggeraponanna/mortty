use crate::renderer::WgpuState;
use log::{error, info};
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};
use std::sync::Arc;
use crate::pty::Pty;

#[derive(Default)]
pub struct App<'a> {
    pub state: Option<WgpuState<'a>>,
    pub pty: Option<Pty>,
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("mortty - New Gen Terminal")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
            
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            
            self.pty = Some(Pty::new().expect("Failed to spawn PTY subprocess"));
            
            let state = pollster::block_on(WgpuState::new(window));
            self.state = Some(state);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = match self.state.as_mut() {
            Some(s) if s.window().id() == window_id => s,
            _ => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                info!("Close requested, shutting down...");
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                state.resize(physical_size);
                state.window().request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Some(pty) = &self.pty {
                    while let Ok(bytes) = pty.rx.try_recv() {
                        if let Ok(text) = String::from_utf8(bytes.clone()) {
                            print!("{}", text);
                        } else {
                            print!("{:?}", bytes);
                        }
                    }
                }

                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        error!("Out of Memory");
                        event_loop.exit();
                    }
                    Err(e) => error!("{:?}", e),
                }
            }
            _ => {}
        }
    }
}

pub fn run() -> Result<(), EventLoopError> {
    info!("Starting mortty...");
    
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app)
}
