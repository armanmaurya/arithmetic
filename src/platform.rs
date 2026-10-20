use std::process::Command;
use std::thread;
use std::time::Duration;

/// Ensures the calculator window starts floating (412x800, centered) in Hyprland
/// while remaining freely resizable afterwards.
pub fn configure_hyprland_floating() {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        return;
    }

    // 1. Register Hyprland 0.56+ Lua window_rule BEFORE the Wayland surface is created
    //    so Hyprland maps the window floating & centered on the very first frame.
    let _ = Command::new("hyprctl")
        .args([
            "eval",
            r#"hl.window_rule({ match = { title = "Arithmetic" }, float = true, size = "412 800", center = true })"#,
        ])
        .output();

    // 2. Fallback watcher: once the window with our PID is mapped, verify it is floating;
    //    if not, float, size (412x800), and center it via hyprctl dispatch.
    let pid = std::process::id();
    thread::spawn(move || {
        let needle = format!("pid: {pid}");
        for _ in 0..40 {
            thread::sleep(Duration::from_millis(25));
            let Ok(out) = Command::new("hyprctl").arg("clients").output() else {
                continue;
            };
            let text = String::from_utf8_lossy(&out.stdout);
            for block in text.split("\n\n") {
                if block.contains(&needle) && block.contains("mapped: 1") {
                    if block.contains("floating: 0") {
                        let _ = Command::new("hyprctl")
                            .args([
                                "--batch",
                                &format!(
                                    "dispatch setfloating pid:{pid}; dispatch resizewindowpixel exact 412 800,pid:{pid}; dispatch centerwindow"
                                ),
                            ])
                            .output();
                    }
                    return;
                }
            }
        }
    });
}
