use crate::core::paths::{app_config_dir, config_file_path};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    if let Ok(json) = serde_json::to_string_pretty(settings)
        && let Some(cfg_dir) = app_config_dir()
    {
        let _ = fs::write(cfg_dir.join("config.json"), json);
    }
}
