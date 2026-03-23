mod pty;
mod renderer;
mod terminal;
mod window;
mod config;

fn main() {
    env_logger::init();
    if let Err(e) = window::run() {
        log::error!("Error running application: {:?}", e);
    }
}
