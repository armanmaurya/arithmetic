use crate::engine::{evaluate_expression, format_number, has_operator_or_func, pop_last_token};
use crate::storage::{clear_saved_history, load_history, load_settings, save_history, save_settings};
use crate::{AppWindow, HistoryEntry, SkinDialog};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;

pub fn skin_window_size(skin_id: &str, variant_id: &str) -> (f32, f32) {
    let skins = crate::skin::discover_skins();
    if let Some(s) = skins.iter().find(|s| s.manifest.id == skin_id) {
        let v = s.active_variant(Some(variant_id));
        return (v.width, v.height);
    }
    (412.0, 780.0)
}

fn clamp_char_boundary(s: &str, byte_offset: usize) -> usize {
    let mut idx = byte_offset.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn set_expr_and_cursor(ui: &AppWindow, new_expr: String, new_cursor: usize) {
    let clamped = clamp_char_boundary(&new_expr, new_cursor) as i32;
    ui.set_expression(new_expr.into());
    ui.set_cursor_byte_offset(clamped);
    ui.invoke_set_cursor(clamped);
}

fn update_live_preview(ui: &AppWindow) {
    let expr = ui.get_expression().to_string();
    ui.set_is_error(false);

    if expr.is_empty() || !has_operator_or_func(&expr) {
        ui.set_preview_result(SharedString::default());
        return;
    }

    if let Some(last) = expr.chars().last()
        && matches!(last, '+' | '−' | '-' | '×' | '*' | '÷' | '/' | '^' | '(')
    {
        ui.set_preview_result(SharedString::default());
        return;
    }

    match evaluate_expression(&expr, ui.get_use_radians()) {
        Ok(val) => {
            let formatted = format_number(val);
            if formatted != expr {
                ui.set_preview_result(formatted.into());
            } else {
                ui.set_preview_result(SharedString::default());
            }
        }
        Err(_) => {
            ui.set_preview_result(SharedString::default());
        }
    }
}

pub fn refresh_skins_state(ui: &AppWindow, dialog: &SkinDialog) -> bool {
    let discovered = crate::skin::discover_skins();
    let has_skins = !discovered.is_empty();

    let mut settings = load_settings();

    let skin_models: Vec<crate::SkinModelEntry> = discovered
        .iter()
        .map(|s| {
            let active_var = if s.manifest.id == settings.active_skin {
                settings.active_variant.clone()
            } else {
                s.manifest.default_variant.clone()
            };
            let variants: Vec<crate::SkinVariantEntry> = s
                .manifest
                .variants
                .iter()
                .map(|v| crate::SkinVariantEntry {
                    id: v.id.clone().into(),
                    name: v.name.clone().into(),
                    width: v.width,
                    height: v.height,
                })
                .collect();
            let has_vars = variants.len() > 1;
            crate::SkinModelEntry {
                id: s.manifest.id.clone().into(),
                name: s.manifest.name.clone().into(),
                description: s.manifest.description.clone().into(),
                author: s.manifest.author.clone().into(),
                has_variants: has_vars,
                active_variant: active_var.into(),
                variants: ModelRc::new(VecModel::from(variants)),
            }
        })
        .collect();
    dialog.set_available_skins(ModelRc::new(VecModel::from(skin_models)));

    if has_skins {
        ui.set_has_skins(true);
        let target_skin = if discovered.iter().any(|s| s.manifest.id == settings.active_skin) {
            settings.active_skin.clone()
        } else {
            let first_id = discovered[0].manifest.id.clone();
            settings.active_skin = first_id.clone();
            settings.active_variant = discovered[0].manifest.default_variant.clone();
            save_settings(&settings);
            first_id
        };

        let target_variant = settings.active_variant.clone();
        let (w, h) = skin_window_size(&target_skin, &target_variant);

        ui.set_active_skin(target_skin.clone().into());
        ui.set_active_variant(target_variant.clone().into());
        ui.set_custom_skin_w(w);
        ui.set_custom_skin_h(h);
        ui.window().set_size(slint::LogicalSize::new(w, h));

        dialog.set_selected_skin(target_skin.into());
        dialog.set_selected_variant(target_variant.into());
        true
    } else {
        ui.set_has_skins(false);
        ui.set_active_skin("".into());
        ui.set_active_variant("".into());
        let (w, h) = (460.0, 340.0);
        ui.set_custom_skin_w(w);
        ui.set_custom_skin_h(h);
        ui.window().set_size(slint::LogicalSize::new(w, h));

        dialog.set_selected_skin("".into());
        dialog.set_selected_variant("".into());
        false
    }
}

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

    // 5b. Close / Hide Main Window (triggered by close button or Casio SHIFT+AC OFF)
    {
        let ui_handle = ui.as_weak();
        ui.on_close_window(move || {
            if let Some(ui) = ui_handle.upgrade() {
                let _ = ui.window().hide();
            }
        });
    }

    // 5c. Open Skins Folder in user file manager
    {
        ui.on_open_skins_folder(move || {
            if let Some(cfg_dir) = crate::storage::app_config_dir() {
                let skins_dir = cfg_dir.join("skins");
                let _ = std::fs::create_dir_all(&skins_dir);
                let _ = std::process::Command::new("xdg-open")
                    .arg(&skins_dir)
                    .spawn();
            }
        });
    }

    // 5d. Reload skins when user clicks "Check Again"
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

    // 6. Apply Skin from Separate Window
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

    // 7. Close Separate Window
    {
        let dialog_handle = dialog.as_weak();
        dialog.on_close_dialog(move || {
            if let Some(dlg) = dialog_handle.upgrade() {
                let _ = dlg.hide();
            }
        });
    }

    // 8. Quick Skin Switch Callbacks
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

    // 10. Main Keypad & Button Handler
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

            match key_str {
                "AC" => {
                    set_expr_and_cursor(&ui, String::new(), 0);
                    ui.set_preview_result(SharedString::default());
                    ui.set_is_error(false);
                    ui.set_just_evaluated(false);
                    return;
                }
                "BACKSPACE" => {
                    if just_eval {
                        ui.set_just_evaluated(false);
                    }
                    let (before, after) = expr.split_at(cursor);
                    let mut before_str = before.to_string();
                    pop_last_token(&mut before_str);
                    let new_cursor = before_str.len();
                    let new_expr = format!("{before_str}{after}");
                    set_expr_and_cursor(&ui, new_expr, new_cursor);
                    update_live_preview(&ui);
                    return;
                }
                "=" => {
                    if expr.is_empty() {
                        return;
                    }
                    match evaluate_expression(&expr, ui.get_use_radians()) {
                        Ok(val) => {
                            let formatted = format_number(val);
                            if has_operator_or_func(&expr) {
                                history.push(HistoryEntry {
                                    expression: expr.clone().into(),
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
                    return;
                }
                "()" => {
                    let (mut before_str, after_str) = if just_eval && cursor == expr.len() {
                        (String::new(), "")
                    } else {
                        let (b, a) = expr.split_at(cursor);
                        (b.to_string(), a)
                    };
                    let open_count = before_str.chars().filter(|&c| c == '(').count();
                    let close_count = before_str.chars().filter(|&c| c == ')').count();
                    let last_char = before_str.chars().last();
                    let can_close = open_count > close_count
                        && matches!(
                            last_char,
                            Some('0'..='9' | ')' | 'π' | 'e' | '%' | '!')
                        );
                    if can_close {
                        before_str.push(')');
                    } else {
                        before_str.push('(');
                    }
                    let new_cursor = before_str.len();
                    let new_expr = format!("{before_str}{after_str}");
                    ui.set_just_evaluated(false);
                    set_expr_and_cursor(&ui, new_expr, new_cursor);
                    update_live_preview(&ui);
                    return;
                }
                _ => {}
            }

            let is_binary_op = matches!(key_str, "+" | "−" | "×" | "÷" | "^" | "^2");
            let is_digit_or_dot =
                key_str.len() == 1 && key_str.chars().all(|c| c.is_ascii_digit() || c == '.');

            let (mut before_str, after_str) = if just_eval
                && cursor == expr.len()
                && (is_digit_or_dot || key_str.ends_with('(') || key_str == "π" || key_str == "e")
            {
                (String::new(), "")
            } else {
                let (b, a) = expr.split_at(cursor);
                (b.to_string(), a)
            };
            ui.set_just_evaluated(false);

            if is_binary_op {
                if before_str.is_empty() {
                    if key_str == "−" {
                        before_str.push('−');
                    }
                } else if let Some(last) = before_str.chars().last() {
                    if matches!(last, '+' | '−' | '×' | '÷' | '^') {
                        before_str.pop();
                        before_str.push_str(key_str);
                    } else if last != '(' || key_str == "−" {
                        before_str.push_str(key_str);
                    }
                }
            } else if key_str == "." {
                let last_number_segment = before_str
                    .rsplit(|c: char| !c.is_ascii_digit() && c != '.')
                    .next()
                    .unwrap_or("");
                if !last_number_segment.contains('.') {
                    if last_number_segment.is_empty() {
                        before_str.push('0');
                    }
                    before_str.push('.');
                }
            } else {
                before_str.push_str(key_str);
            }

            let new_cursor = before_str.len();
            let new_expr = format!("{before_str}{after_str}");
            set_expr_and_cursor(&ui, new_expr, new_cursor);
            update_live_preview(&ui);
        });
    }
}
