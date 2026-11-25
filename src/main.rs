// Prevent console window in addition to Slint window in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use arithmetic::cli;
use arithmetic::ui::App;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(code) = cli::run_from_args(std::env::args()) {
        std::process::exit(code);
    }
    let app = App::new()?;
    app.start();
    slint::run_event_loop_until_quit()?;
    Ok(())
}
