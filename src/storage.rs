use crate::HistoryEntry;
use slint::{Model, VecModel};
use std::fs;
use std::path::PathBuf;

fn history_file_path() -> Option<PathBuf> {
    let base_dir = if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(xdg)
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".local").join("share")
    } else {
        return None;
    };

    let app_dir = base_dir.join("arithmetic");
    let _ = fs::create_dir_all(&app_dir);
    Some(app_dir.join("history.tsv"))
}

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
