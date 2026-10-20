// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod controller;
mod engine;
mod platform;
mod storage;

use std::error::Error;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    platform::configure_hyprland_floating();

    let ui = AppWindow::new()?;
    controller::setup_callbacks(&ui);

    ui.run()?;

    Ok(())
}
