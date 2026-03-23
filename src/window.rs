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
use crate::terminal::Terminal;
use vte::Parser;

pub struct App<'a> {
    pub state: Option<WgpuState<'a>>,
    pub pty: Option<Pty>,
    pub terminal: Terminal,
    pub parser: Parser,
    pub proxy: winit::event_loop::EventLoopProxy<()>,
}


impl<'a> ApplicationHandler for App<'a> {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        if let Some(state) = &self.state {
            state.window().request_redraw();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("mortty - New Gen Terminal")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
            
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            
            self.pty = Some(Pty::new(self.proxy.clone()).expect("Failed to spawn PTY subprocess"));
            
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
                if let Some(pty) = &mut self.pty {
                    let mut has_output = false;
                    while let Ok(bytes) = pty.rx.try_recv() {
                        for byte in bytes.iter() {
                            self.parser.advance(&mut self.terminal, *byte);
                        }
                        
                        // Debug print to console alongside terminal grid parsing
                        if let Ok(text) = String::from_utf8(bytes.clone()) {
                            print!("{}", text);
                        }
                        has_output = true;
                    }
                    if has_output {
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                    }
                }

                match state.render(&self.terminal) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        error!("Out of Memory");
                        event_loop.exit();
                    }
                    Err(e) => error!("{:?}", e),
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    if let Some(pty) = &mut self.pty {
                        if let Some(text) = &event.text {
                            let _ = pty.write(text.as_bytes());
                        } else {
                            use winit::keyboard::{Key, NamedKey};
                            match &event.logical_key {
                                Key::Named(NamedKey::Enter) => { let _ = pty.write(b"\r"); },
                                Key::Named(NamedKey::Backspace) => { let _ = pty.write(b"\x7f"); },
                                Key::Named(NamedKey::Escape) => { let _ = pty.write(b"\x1b"); },
                                Key::Named(NamedKey::Tab) => { let _ = pty.write(b"\t"); },
                                Key::Named(NamedKey::ArrowUp) => { let _ = pty.write(b"\x1b[A"); },
                                Key::Named(NamedKey::ArrowDown) => { let _ = pty.write(b"\x1b[B"); },
                                Key::Named(NamedKey::ArrowRight) => { let _ = pty.write(b"\x1b[C"); },
                                Key::Named(NamedKey::ArrowLeft) => { let _ = pty.write(b"\x1b[D"); },
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn run() -> Result<(), EventLoopError> {
    info!("Starting mortty...");
    
    let event_loop = EventLoop::<()>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    let mut app = App {
        state: None,
        pty: None,
        terminal: Terminal::new(80, 24),
        parser: Parser::new(),
        proxy,
    };
    event_loop.run_app(&mut app)
}
