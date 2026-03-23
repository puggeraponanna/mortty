use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::io::{Read, Write};

#[derive(Debug)]
pub enum ControlEvent {
    Wakeup,
    PtyExit,
}

pub struct Pty {
    pub pty_pair: PtyPair,
    pub rx: Receiver<Vec<u8>>,
    pub writer: Box<dyn std::io::Write + Send>,
    pub proxy_pending: Arc<AtomicBool>,
}

impl Pty {
    pub fn new(proxy: winit::event_loop::EventLoopProxy<ControlEvent>, cols: u16, rows: u16) -> Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Determine the shell to run
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let mut cmd = CommandBuilder::new(shell);
        
        let term = std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string());
        cmd.env("TERM", term);
        
        let colorterm = std::env::var("COLORTERM").unwrap_or_else(|_| "truecolor".to_string());
        cmd.env("COLORTERM", colorterm);

        // Spawn the child process linked to the PTY
        let _child = pair.slave.spawn_command(cmd)?;

        // Set up a channel to send read bytes to the main thread
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
        
        let writer = pair.master.take_writer()?;
        let mut reader = pair.master.try_clone_reader()?;

        let proxy_pending = Arc::new(AtomicBool::new(false));
        let proxy_pending_thread = proxy_pending.clone();

        // Spawn a background thread to continually read from the PTY
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = proxy.send_event(ControlEvent::PtyExit);
                        break; // EOF
                    }
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break; // Receiver hung up
                        }
                        // Only send Wakeup if one isn't already pending in the loop
                        if !proxy_pending_thread.swap(true, Ordering::SeqCst) {
                            let _ = proxy.send_event(ControlEvent::Wakeup);
                        }
                    }
                    Err(e) => {
                        log::error!("Error reading from PTY: {}", e);
                        let _ = proxy.send_event(ControlEvent::PtyExit);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            pty_pair: pair,
            rx,
            writer,
            proxy_pending,
        })
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.pty_pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }
}
