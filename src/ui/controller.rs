use super::state::{refresh_skins_state, set_expr_and_cursor, skin_window_size, update_live_preview};
use crate::core::editor::{apply_key_press, clamp_char_boundary, has_operator_or_func, KeyAction};
use crate::core::engine::{evaluate_expression, format_number};
use crate::core::paths::app_config_dir;
use crate::storage::{
    clear_saved_history, load_history, load_settings, save_history, save_settings,
};
use crate::{AppWindow, HistoryEntry, SkinDialog};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;

pub fn setup_callbacks(ui: &AppWindow, dialog: &SkinDialog) {
    let history_model = Rc::new(VecModel::<HistoryEntry>::from(load_history()));
    ui.set_history(ModelRc::from(history_model.clone()));

    refresh_skins_state(ui, dialog);

    // 1. Direct keyboard edits inside TextInput
    {
        let ui_handle = ui.as_weak();
        ui.on_expression_edited(move || {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_just_evaluated(false);
                update_live_preview(&ui);
            }
        });
    }

    // 2. Toggle DEG / RAD
    {
        let ui_handle = ui.as_weak();
        ui.on_toggle_angle_unit(move || {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_use_radians(!ui.get_use_radians());
                update_live_preview(&ui);
            }
        });
    }

    // 3. Clear History
    {
        let history = history_model.clone();
        ui.on_clear_history(move || {
            history.clear();
            clear_saved_history();
        });
    }

    // 4. Select History Entry
    {
        let ui_handle = ui.as_weak();
        ui.on_select_history_item(move |val| {
            if let Some(ui) = ui_handle.upgrade() {
                let s = val.to_string();
                let len = s.len();
                set_expr_and_cursor(&ui, s, len);
                ui.set_just_evaluated(false);
                update_live_preview(&ui);
            }
        });
    }

    // 5. Open Separate Skin & Theme Selection Window
    {
        let dialog_handle = dialog.as_weak();
        ui.on_open_skin_dialog(move || {
            if let Some(dlg) = dialog_handle.upgrade() {
                if let Err(e) = dlg.show() {
                    eprintln!("Failed to show Appearance & Themes dialog: {e:?}");
                }
                dlg.window().request_redraw();
            }
        });
    }

    // 6. Close / Hide Main Window
    {
        let ui_handle = ui.as_weak();
        ui.on_close_window(move || {
            if let Some(ui) = ui_handle.upgrade() {
                let _ = ui.window().hide();
            }
        });
    }

    // 7. Open Skins Folder in user file manager
    {
        ui.on_open_skins_folder(move || {
            if let Some(cfg_dir) = app_config_dir() {
                let skins_dir = cfg_dir.join("skins");
                let _ = std::fs::create_dir_all(&skins_dir);
                let _ = std::process::Command::new("xdg-open")
                    .arg(&skins_dir)
                    .spawn();
            }
        });
    }

    // 8. Reload skins when user clicks "Check Again"
    {
        let ui_handle = ui.as_weak();
        let dialog_handle = dialog.as_weak();
        ui.on_reload_skins(move || {
            if let (Some(ui), Some(dlg)) = (ui_handle.upgrade(), dialog_handle.upgrade()) {
                refresh_skins_state(&ui, &dlg);
            }
        });
    }

    // Ensure closing via window manager (X button) hides the dialog rather than destroying it
    dialog.window().on_close_requested(move || {
        slint::CloseRequestResponse::HideWindow
    });

    // 9. Apply Skin from Separate Window
    {
        let ui_handle = ui.as_weak();
        let dialog_handle = dialog.as_weak();
        dialog.on_apply_skin(move |skin_id, variant_id| {
            if let Some(ui) = ui_handle.upgrade() {
                let skin_str = skin_id.as_str();
                let variant_str = variant_id.as_str();
                ui.set_active_skin(skin_id.clone());
                ui.set_active_variant(variant_id.clone());
                if let Some(dlg) = dialog_handle.upgrade() {
                    dlg.set_selected_skin(skin_id.clone());
                    dlg.set_selected_variant(variant_id.clone());
                }
                let (w, h) = skin_window_size(skin_str, variant_str);
                ui.set_custom_skin_w(w);
                ui.set_custom_skin_h(h);
                ui.window().set_size(slint::LogicalSize::new(w, h));
                let mut settings = load_settings();
                settings.active_skin = skin_str.to_string();
                settings.active_variant = variant_str.to_string();
                save_settings(&settings);
            }
        });
    }

    // 10. Close Separate Window
    {
        let dialog_handle = dialog.as_weak();
        dialog.on_close_dialog(move || {
            if let Some(dlg) = dialog_handle.upgrade() {
                let _ = dlg.hide();
            }
        });
    }

    // 11. Quick Skin Switch Callbacks
    {
        let ui_handle = ui.as_weak();
        let dialog_handle = dialog.as_weak();
        ui.on_switch_skin(move |target_skin| {
            if let Some(ui) = ui_handle.upgrade() {
                let skin_str = target_skin.as_str();
                let variant_str = "standard";
                ui.set_active_skin(target_skin.clone());
                ui.set_active_variant(variant_str.into());
                if let Some(dlg) = dialog_handle.upgrade() {
                    dlg.set_selected_skin(target_skin.clone());
                    dlg.set_selected_variant(variant_str.into());
                }
                let (w, h) = skin_window_size(skin_str, variant_str);
                ui.set_custom_skin_w(w);
                ui.set_custom_skin_h(h);
                ui.window().set_size(slint::LogicalSize::new(w, h));
                let mut settings = load_settings();
                settings.active_skin = skin_str.to_string();
                settings.active_variant = variant_str.to_string();
                save_settings(&settings);
            }
        });
    }

    // 12. Main Keypad & Button Handler
    {
        let ui_handle = ui.as_weak();
        let history = history_model;
        ui.on_press_key(move |key| {
            let Some(ui) = ui_handle.upgrade() else {
                return;
            };
            ui.set_expr_scroll_offset(0.0);
            let key_str = key.as_str();
            let expr = ui.get_expression().to_string();
            let cursor = clamp_char_boundary(&expr, ui.get_cursor_byte_offset().max(0) as usize);
            let just_eval = ui.get_just_evaluated();

            match apply_key_press(&expr, cursor, key_str, just_eval) {
                KeyAction::Clear => {
                    set_expr_and_cursor(&ui, String::new(), 0);
                    ui.set_preview_result(SharedString::default());
                    ui.set_is_error(false);
                    ui.set_just_evaluated(false);
                }
                KeyAction::Evaluate => {
                    if expr.is_empty() {
                        return;
                    }
                    match evaluate_expression(&expr, ui.get_use_radians()) {
                        Ok(val) => {
                            let formatted = format_number(val);
                            if has_operator_or_func(&expr) {
                                history.push(HistoryEntry {
                                    expression: expr.into(),
                                    result: formatted.clone().into(),
                                });
                                save_history(&history);
                            }
                            let new_len = formatted.len();
                            set_expr_and_cursor(&ui, formatted, new_len);
                            ui.set_preview_result(SharedString::default());
                            ui.set_is_error(false);
                            ui.set_just_evaluated(true);
                        }
                        Err(err_msg) => {
                            if !err_msg.is_empty() {
                                ui.set_preview_result(err_msg.into());
                                ui.set_is_error(true);
                            }
                        }
                    }
                }
                KeyAction::UpdateExpression {
                    new_expression,
                    new_cursor,
                } => {
                    ui.set_just_evaluated(false);
                    set_expr_and_cursor(&ui, new_expression, new_cursor);
                    update_live_preview(&ui);
                }
            }
        });
    }
}
