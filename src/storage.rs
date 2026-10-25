use crate::HistoryEntry;
use slint::{Model, VecModel};
use std::fs;
use std::path::PathBuf;

fn is_dir_writable(p: &std::path::Path) -> bool {
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

fn history_file_path() -> Option<PathBuf> {
    // Prefer ~/.config/arithmetic/history.tsv, fallback to data dir
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

#[allow(dead_code)]
pub fn load_history() -> Vec<HistoryEntry> {
    let Some(path) = history_file_path() else {
        return Vec::new();
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };

    contents
        .lines()
        .filter_map(|line| {
            let (expr, res) = line.split_once('\t')?;
            if expr.is_empty() || res.is_empty() {
                return None;
            }
            Some(HistoryEntry {
                expression: expr.into(),
                result: res.into(),
            })
        })
        .collect()
}

pub fn load_raw_history() -> Vec<(String, String)> {
    let Some(path) = history_file_path() else {
        return Vec::new();
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };

    contents
        .lines()
        .filter_map(|line| {
            let (expr, res) = line.split_once('\t')?;
            if expr.is_empty() || res.is_empty() {
                return None;
            }
            Some((expr.to_string(), res.to_string()))
        })
        .collect()
}

pub fn append_history_entry(expr: &str, res: &str) {
    let Some(path) = history_file_path() else {
        return;
    };
    use std::io::Write;
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let clean_expr = expr.replace(['\t', '\n', '\r'], " ");
        let clean_res = res.replace(['\t', '\n', '\r'], " ");
        let _ = writeln!(file, "{clean_expr}\t{clean_res}");
    }
}

#[allow(dead_code)]
pub fn save_history(model: &VecModel<HistoryEntry>) {
    let Some(path) = history_file_path() else {
        return;
    };

    let mut out = String::new();
    for entry in model.iter() {
        let expr = entry.expression.as_str().replace(['\t', '\n', '\r'], " ");
        let res = entry.result.as_str().replace(['\t', '\n', '\r'], " ");
        out.push_str(&expr);
        out.push('\t');
        out.push_str(&res);
        out.push('\n');
    }

    let _ = fs::write(path, out);
}

pub fn clear_saved_history() {
    if let Some(path) = history_file_path() {
        let _ = fs::remove_file(path);
    }
}


pub fn config_file_path() -> Option<PathBuf> {
    app_config_dir().map(|d| d.join("config.json"))
}

pub fn settings_file_path() -> Option<PathBuf> {
    config_file_path()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_active_skin")]
    pub active_skin: String,
    #[serde(default = "default_active_variant")]
    pub active_variant: String,
    #[serde(default = "default_theme_mode")]
    pub theme_mode: i32,
}

fn default_active_skin() -> String {
    "".to_string()
}

fn default_active_variant() -> String {
    "".to_string()
}

fn default_theme_mode() -> i32 {
    2
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            active_skin: default_active_skin(),
            active_variant: default_active_variant(),
            theme_mode: default_theme_mode(),
        }
    }
}

pub fn load_settings() -> AppSettings {
    let Some(path) = config_file_path() else {
        return AppSettings::default();
    };
    let Ok(data) = fs::read_to_string(path) else {
        return AppSettings::default();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_settings(settings: &AppSettings) {
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        // Save strictly to ~/.config/arithmetic/config.json
        if let Some(cfg_dir) = app_config_dir() {
            let _ = fs::write(cfg_dir.join("config.json"), &json);
        }
    }
}
