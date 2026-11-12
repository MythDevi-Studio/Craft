//! Moce — a modern code editor built with Rust and Skia.
//!
//! Entry point: initialise logging, build the event loop, and hand control
//! over to `MoceApp`.

#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod editor;
mod lsp;
mod math;
mod syntax;
mod ui;

use app::MoceApp;
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = MoceApp::new();
    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("Event loop error: {}", e);
    }
}
