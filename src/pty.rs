use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::io::Read;

pub struct Pty {
    pub pty_pair: PtyPair,
    pub rx: Receiver<Vec<u8>>,
}

impl Pty {
    pub fn new() -> Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Determine the shell to run
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let cmd = CommandBuilder::new(shell);

        // Spawn the child process linked to the PTY
        let _child = pair.slave.spawn_command(cmd)?;

        // Set up a channel to send read bytes to the main thread
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
        
        let mut reader = pair.master.try_clone_reader()?;

        // Spawn a background thread to continually read from the PTY
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break; // Receiver hung up
                        }
                    }
                    Err(e) => {
                        log::error!("Error reading from PTY: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            pty_pair: pair,
            rx,
        })
    }
}
