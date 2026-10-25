// Prevent console window in addition to Slint window in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use arithmetic::controller;
use arithmetic::storage;
use arithmetic::{AppWindow, M3Theme, SkinDialog};
use slint::ComponentHandle;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let settings = storage::load_settings();

    let ui = AppWindow::new()?;
    let dialog = SkinDialog::new()?;

    // Apply saved theme mode
    ui.global::<M3Theme>().set_theme_mode(settings.theme_mode);
    dialog.global::<M3Theme>().set_theme_mode(settings.theme_mode);

    controller::setup_callbacks(&ui, &dialog);

    ui.run()?;

    Ok(())
}
