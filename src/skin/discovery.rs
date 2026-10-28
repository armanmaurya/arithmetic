use super::manifest::{SkinInfo, SkinManifest};
use crate::core::paths::{app_config_dir, app_data_dir};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

pub fn skin_search_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 0. Debug builds load the repository's own skins first, so `cargo run`
    //    always reflects the skins being developed.
    #[cfg(debug_assertions)]
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("skins"));

    // 1. User config directory: $XDG_CONFIG_HOME/arithmetic/skins or ~/.config/arithmetic/skins
    if let Some(cfg_dir) = app_config_dir() {
        dirs.push(cfg_dir.join("skins"));
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        dirs.push(PathBuf::from(xdg).join("arithmetic").join("skins"));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".config").join("arithmetic").join("skins"));
    }

    // 2. User data directory: $XDG_DATA_HOME/arithmetic/skins or ~/.local/share/arithmetic/skins
    if let Some(data_dir) = app_data_dir() {
        dirs.push(data_dir.join("skins"));
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(xdg).join("arithmetic").join("skins"));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local").join("share").join("arithmetic").join("skins"));
    }

    // 3. Executable directory: <exe_dir>/skins and <exe_dir>/../share/arithmetic/skins
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        dirs.push(parent.join("skins"));
        dirs.push(parent.join("..").join("share").join("arithmetic").join("skins"));
    }

    // AppImage: bundled skins are unpacked at the AppDir root
    if let Some(appdir) = std::env::var_os("APPDIR") {
        dirs.push(PathBuf::from(appdir).join("skins"));
    }

    // 4. System directories: /usr/share/arithmetic/skins and /usr/local/share/arithmetic/skins
    dirs.push(PathBuf::from("/usr/share/arithmetic/skins"));
    dirs.push(PathBuf::from("/usr/local/share/arithmetic/skins"));

    // 5. In test or development mode (ARITHMETIC_DEV=1 or cargo test), also check ./skins
    #[cfg(test)]
    {
        if let Ok(cwd) = std::env::current_dir() {
            dirs.push(cwd.join("skins"));
        }
    }
    #[cfg(not(test))]
    {
        if std::env::var_os("ARITHMETIC_DEV").is_some()
            && let Ok(cwd) = std::env::current_dir()
        {
            dirs.push(cwd.join("skins"));
        }
    }

    // The XDG fallbacks usually resolve to the same place as the app dirs;
    // scan each directory once.
    let mut seen = HashSet::new();
    dirs.retain(|d| seen.insert(d.canonicalize().unwrap_or_else(|_| d.clone())));
    dirs
}

pub fn discover_skins() -> Vec<SkinInfo> {
    let mut discovered = Vec::new();
    let mut seen_ids = HashSet::new();

    for dir in skin_search_directories() {
        if !dir.is_dir() {
            continue;
        }

        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let skin_dir = entry.path();
            if !skin_dir.is_dir() {
                continue;
            }

            let manifest_path = skin_dir.join("manifest.json");
            if !manifest_path.is_file() {
                continue;
            }

            let Ok(content) = fs::read_to_string(&manifest_path) else {
                continue;
            };

            let manifest = match serde_json::from_str::<SkinManifest>(&content) {
                Ok(manifest) => manifest,
                Err(e) => {
                    eprintln!("[skins] Skipping {:?}: {e}", manifest_path);
                    continue;
                }
            };

            if manifest.layouts.is_empty() {
                eprintln!("[skins] Manifest at {:?} has no layouts", manifest_path);
                continue;
            }

            if seen_ids.contains(&manifest.id) {
                continue;
            }

            let def_layout = manifest
                .layouts
                .iter()
                .find(|l| l.id == manifest.default_layout)
                .unwrap_or(&manifest.layouts[0]);
            let main_file = skin_dir.join(&def_layout.file);
            if !main_file.is_file() {
                eprintln!(
                    "[skins] Default layout file missing for skin {}: {:?}",
                    manifest.id, main_file
                );
                continue;
            }

            seen_ids.insert(manifest.id.clone());
            discovered.push(SkinInfo {
                manifest,
                directory: skin_dir,
                main_file,
            });
        }
    }

    discovered
}
