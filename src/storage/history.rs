use crate::core::paths::history_file_path;
use std::fs;
use std::io::Write;

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
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let clean_expr = expr.replace(['\t', '\n', '\r'], " ");
        let clean_res = res.replace(['\t', '\n', '\r'], " ");
        let _ = writeln!(file, "{clean_expr}\t{clean_res}");
    }
}

pub fn clear_saved_history() {
    if let Some(path) = history_file_path() {
        let _ = fs::remove_file(path);
    }
}
