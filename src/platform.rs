/// Platform-specific window management hooks.
/// Window sizing, presentation, and floating geometries are handled natively by Slint and Wayland.

pub fn configure_hyprland_floating(_initial_w: i32, _initial_h: i32) {
    // No-op: Window geometry is managed natively by Slint & Wayland xdg-shell.
}

pub fn focus_theme_dialog() {
    // No-op: Slint's Window::show() handles raising and focusing windows.
}

pub fn resize_hyprland_floating(_w: i32, _h: i32) {
    // No-op: Slint's Window::set_size() dynamically reconfigures the window surface.
}
