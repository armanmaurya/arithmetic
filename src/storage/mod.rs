pub mod history;
pub mod settings;

pub use history::{append_history_entry, clear_saved_history, load_raw_history};
pub use settings::{load_settings, save_settings, AppSettings};

use crate::HistoryEntry;
use slint::{Model, VecModel};

pub fn load_history() -> Vec<HistoryEntry> {
    load_raw_history()
        .into_iter()
        .map(|(expr, res)| HistoryEntry {
            expression: expr.into(),
            result: res.into(),
        })
        .collect()
}

pub fn save_history(model: &VecModel<HistoryEntry>) {
    let Some(path) = crate::core::paths::history_file_path() else {
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

    let _ = std::fs::write(path, out);
}
