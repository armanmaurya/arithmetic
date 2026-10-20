use crate::engine::{evaluate_expression, format_number, has_operator_or_func, pop_last_token};
use crate::storage::{clear_saved_history, load_history, save_history};
use crate::{AppWindow, HistoryEntry};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;

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

pub fn setup_callbacks(ui: &AppWindow) {
    let history_model = Rc::new(VecModel::<HistoryEntry>::from(load_history()));
    ui.set_history(ModelRc::from(history_model.clone()));

    // Direct keyboard edits inside TextInput
    {
        let ui_handle = ui.as_weak();
        ui.on_expression_edited(move || {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_just_evaluated(false);
                update_live_preview(&ui);
            }
        });
    }

    // Toggle DEG / RAD
    {
        let ui_handle = ui.as_weak();
        ui.on_toggle_angle_unit(move || {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_use_radians(!ui.get_use_radians());
                update_live_preview(&ui);
            }
        });
    }

    // Clear History
    {
        let history = history_model.clone();
        ui.on_clear_history(move || {
            history.clear();
            clear_saved_history();
        });
    }

    // Select History Entry (inserts expression or result at end and places cursor there)
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

    // Main Keypad Handler (Caret-aware insertion & deletion)
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

            // If user just pressed '=' and caret is still at the end, typing a new number starts fresh;
            // if user moved the caret inside the result, insert at the custom caret position!
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
