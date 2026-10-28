use crate::core::editor::{clamp_char_boundary, has_operator_or_func};
use crate::core::engine::{evaluate_expression, format_number};
use crate::skin::discover_skins;
use crate::storage::{load_settings, save_settings};
use crate::{AppWindow, SkinDialog};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

pub fn skin_window_size(skin_id: &str, variant_id: &str) -> (f32, f32) {
    let skins = discover_skins();
    if let Some(s) = skins.iter().find(|s| s.manifest.id == skin_id) {
        let v = s.active_variant(Some(variant_id));
        return (v.width, v.height);
    }
    (412.0, 780.0)
}

pub fn set_expr_and_cursor(ui: &AppWindow, new_expr: String, new_cursor: usize) {
    let clamped = clamp_char_boundary(&new_expr, new_cursor) as i32;
    ui.set_expression(new_expr.into());
    ui.set_cursor_byte_offset(clamped);
    ui.invoke_set_cursor(clamped);
}

pub fn update_live_preview(ui: &AppWindow) {
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
    let discovered = discover_skins();
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
