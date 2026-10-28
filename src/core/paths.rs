use std::fs;
use std::path::{Path, PathBuf};

pub fn is_dir_writable(p: &Path) -> bool {
    if fs::create_dir_all(p).is_err() {
        return false;
    }
    let test_file = p.join(".write_test");
    if fs::write(&test_file, b"ok").is_ok() {
        let _ = fs::remove_file(test_file);
        true
    } else {
        false
    }
}

pub fn app_config_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let p = PathBuf::from(xdg).join("arithmetic");
        if is_dir_writable(&p) {
            return Some(p);
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let p = PathBuf::from(home).join(".config").join("arithmetic");
        if is_dir_writable(&p) {
            return Some(p);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join(".config").join("arithmetic");
        if is_dir_writable(&p) {
            return Some(p);
        }
    }
    let p = std::env::temp_dir().join("arithmetic").join("config");
    let _ = fs::create_dir_all(&p);
    Some(p)
}

pub fn app_data_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        let p = PathBuf::from(xdg).join("arithmetic");
        if is_dir_writable(&p) {
            return Some(p);
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let p = PathBuf::from(home).join(".local").join("share").join("arithmetic");
        if is_dir_writable(&p) {
            return Some(p);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join(".data");
        if is_dir_writable(&p) {
            return Some(p);
        }
    }
    let p = std::env::temp_dir().join("arithmetic");
    let _ = fs::create_dir_all(&p);
    Some(p)
}

pub fn config_file_path() -> Option<PathBuf> {
    app_config_dir().map(|d| d.join("config.json"))
}

pub fn history_file_path() -> Option<PathBuf> {
    if let Some(cfg) = app_config_dir() {
        let path = cfg.join("history.tsv");
        if path.exists() {
            return Some(path);
        }
    }
    if let Some(data) = app_data_dir() {
        let path = data.join("history.tsv");
        if path.exists() {
            return Some(path);
        }
    }
    app_config_dir().map(|d| d.join("history.tsv"))
}
