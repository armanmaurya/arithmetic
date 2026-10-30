//! Runtime skin host.
//!
//! Skins are standalone `.slint` folders compiled at runtime with the Slint
//! interpreter. The host owns all calculator state and talks to whichever skin
//! is active through the `AppWindow` contract (see README "Writing a skin").
//! Every property and callback except `press-key` is optional: a skin only
//! declares the ones it uses.

use crate::core::editor::{KeyAction, apply_key_press, clamp_char_boundary, has_operator_or_func};
use crate::core::engine::{evaluate_expression, format_number};
use crate::core::paths::app_config_dir;
use crate::skin::{SkinInfo, compile_skin_layout, discover_skins};
use crate::storage::{
    SkinPrefs, append_history_entry, clear_saved_history, load_raw_history, load_settings,
    save_settings,
};
use crate::{NoSkinWindow, SkinDialog, SkinModelEntry, SkinOptionEntry};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use slint_interpreter::{ComponentInstance, SetPropertyError, Struct, Value};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

#[derive(Default)]
struct CalcState {
    expression: String,
    cursor: usize,
    preview: String,
    is_error: bool,
    just_evaluated: bool,
    use_radians: bool,
}

type SkinCallback = Box<dyn Fn(&App, &[Value])>;

struct ActiveSkin {
    id: String,
    layout: String,
    color_theme: String,
    instance: ComponentInstance,
}

pub struct App {
    state: RefCell<CalcState>,
    history: Rc<VecModel<Value>>,
    active: RefCell<Option<ActiveSkin>>,
    /// Skins found by the last scan, in the same order as the picker list.
    skins: RefCell<Vec<(SkinInfo, SkinModelEntry)>>,
    dialog: SkinDialog,
    no_skin: NoSkinWindow,
}

fn history_value(expression: &str, result: &str) -> Value {
    let entry: Struct = [
        ("expression".to_string(), Value::String(expression.into())),
        ("result".to_string(), Value::String(result.into())),
    ]
    .into_iter()
    .collect();
    Value::Struct(entry)
}

/// Sets an optional contract property. Missing properties are fine; a property
/// declared with the wrong type is a skin bug worth reporting.
fn set_opt(inst: &ComponentInstance, name: &str, value: Value) {
    if let Err(err) = inst.set_property(name, value)
        && err != SetPropertyError::NoSuchProperty
    {
        eprintln!("[skins] Property '{name}' has an unexpected type: {err:?}");
    }
}

fn live_preview(expr: &str, use_radians: bool) -> String {
    if expr.is_empty() || !has_operator_or_func(expr) {
        return String::new();
    }
    if let Some(last) = expr.chars().last()
        && matches!(last, '+' | '−' | '-' | '×' | '*' | '÷' | '/' | '^' | '(')
    {
        return String::new();
    }
    match evaluate_expression(expr, use_radians) {
        Ok(val) => {
            let formatted = format_number(val);
            if formatted != expr {
                formatted
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    }
}

fn skin_badge(name: &str) -> String {
    let initials: String = name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect();
    initials.to_uppercase()
}

/// Parses `#RRGGBB` or `#RRGGBBAA`.
fn parse_hex_color(hex: &str) -> Option<slint::Color> {
    let hex = hex.strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    match hex.len() {
        6 => Some(slint::Color::from_rgb_u8(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(slint::Color::from_argb_u8(
            byte(6)?,
            byte(0)?,
            byte(2)?,
            byte(4)?,
        )),
        _ => None,
    }
}

fn load_image(path: Option<std::path::PathBuf>) -> Option<slint::Image> {
    path.and_then(|p| slint::Image::load_from_path(&p).ok())
}

/// Shows one of the app's own windows. Slint does not always draw the first
/// frame of a window shown while another is open; on Wayland a window without
/// a frame is never mapped, so request one explicitly.
fn show_window(component: &impl ComponentHandle) -> Result<(), slint::PlatformError> {
    component.show()?;
    component.window().request_redraw();
    Ok(())
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn skin_entry(skin: &SkinInfo) -> SkinModelEntry {
    let m = &skin.manifest;
    let layouts: Vec<SkinOptionEntry> = m
        .layouts
        .iter()
        .map(|l| SkinOptionEntry {
            id: l.id.clone().into(),
            name: l.name.clone().into(),
            swatch: ModelRc::default(),
        })
        .collect();
    let color_themes: Vec<SkinOptionEntry> = m
        .color_themes
        .iter()
        .map(|t| {
            let swatch: Vec<slint::Color> = t
                .swatch
                .iter()
                .filter_map(|h| parse_hex_color(h))
                .take(3)
                .collect();
            SkinOptionEntry {
                id: t.id.clone().into(),
                name: t.name.clone().into(),
                swatch: ModelRc::new(VecModel::from(swatch)),
            }
        })
        .collect();

    let mut summary = vec![plural(layouts.len().max(1), "layout", "layouts")];
    if !color_themes.is_empty() {
        summary.push(plural(color_themes.len(), "colour", "colours"));
    }
    let thumbnail = load_image(skin.preview_path(None));

    SkinModelEntry {
        id: m.id.clone().into(),
        name: m.name.clone().into(),
        badge: skin_badge(&m.name).into(),
        description: m.description.clone().into(),
        author: m.author.clone().into(),
        summary: summary.join(" · ").into(),
        has_thumbnail: thumbnail.is_some(),
        thumbnail: thumbnail.unwrap_or_default(),
        layouts: ModelRc::new(VecModel::from(layouts)),
        color_themes: ModelRc::new(VecModel::from(color_themes)),
    }
}

fn skins_dir_display() -> String {
    app_config_dir()
        .map(|d| d.join("skins").display().to_string())
        .unwrap_or_else(|| "~/.config/arithmetic/skins".to_string())
}

fn open_skins_folder() {
    if let Some(cfg_dir) = app_config_dir() {
        let skins_dir = cfg_dir.join("skins");
        let _ = std::fs::create_dir_all(&skins_dir);
        let _ = std::process::Command::new("xdg-open")
            .arg(&skins_dir)
            .spawn();
    }
}

impl App {
    pub fn new() -> Result<Rc<Self>, slint::PlatformError> {
        let history: Vec<Value> = load_raw_history()
            .iter()
            .map(|(e, r)| history_value(e, r))
            .collect();

        let app = Rc::new(Self {
            state: RefCell::new(CalcState::default()),
            history: Rc::new(VecModel::from(history)),
            active: RefCell::new(None),
            skins: RefCell::new(Vec::new()),
            dialog: SkinDialog::new()?,
            no_skin: NoSkinWindow::new()?,
        });
        app.no_skin.set_skins_dir(skins_dir_display().into());
        app.wire_app_windows();
        Ok(app)
    }

    /// Loads the saved skin, falling back to Material 3 and then any skin that compiles.
    pub fn start(self: &Rc<Self>) {
        let skins = self.refresh_skin_list();
        let settings = load_settings();

        let mut candidates = vec![settings.active_skin, "material-3".to_string()];
        candidates.extend(skins.iter().map(|s| s.manifest.id.clone()));

        let mut last_err = String::new();
        for id in candidates {
            if id.is_empty() {
                continue;
            }
            match self.activate(&id, "", "") {
                Ok(()) => return,
                Err(e) => {
                    eprintln!("[skins] {e}");
                    last_err = e;
                }
            }
        }

        self.no_skin.set_load_error(last_err.into());
        if let Err(e) = show_window(&self.no_skin) {
            eprintln!("Failed to show window: {e:?}");
        }
    }

    fn wire_app_windows(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.dialog.on_apply_skin(move |id, layout, color_theme| {
            if let Some(app) = weak.upgrade() {
                app.apply_selection(id.to_string(), layout.to_string(), color_theme.to_string());
            }
        });

        let weak = Rc::downgrade(self);
        self.dialog.on_reload_skins(move || {
            if let Some(app) = weak.upgrade() {
                app.refresh_skin_list();
                // Recompile the active skin so on-disk edits show up immediately.
                let current = app
                    .active
                    .borrow()
                    .as_ref()
                    .map(|s| (s.id.clone(), s.layout.clone(), s.color_theme.clone()));
                if let Some((id, layout, color_theme)) = current {
                    app.schedule_activate(id, layout, color_theme);
                }
            }
        });

        let weak = Rc::downgrade(self);
        self.dialog.on_close_dialog(move || {
            if let Some(app) = weak.upgrade() {
                let _ = app.dialog.hide();
            }
        });

        self.dialog.on_open_skins_folder(open_skins_folder);
        self.no_skin.on_open_skins_folder(open_skins_folder);

        let weak = Rc::downgrade(self);
        self.no_skin.on_reload_skins(move || {
            if let Some(app) = weak.upgrade() {
                app.start();
            }
        });

        self.dialog
            .window()
            .on_close_requested(|| slint::CloseRequestResponse::HideWindow);
        self.no_skin.window().on_close_requested(|| {
            let _ = slint::quit_event_loop();
            slint::CloseRequestResponse::HideWindow
        });
    }

    fn refresh_skin_list(&self) -> Vec<SkinInfo> {
        let skins = discover_skins();
        let entries: Vec<(SkinInfo, SkinModelEntry)> =
            skins.iter().map(|s| (s.clone(), skin_entry(s))).collect();
        let models: Vec<SkinModelEntry> = entries.iter().map(|(_, e)| e.clone()).collect();
        self.dialog
            .set_available_skins(ModelRc::new(VecModel::from(models)));
        self.skins.replace(entries);
        self.sync_dialog();
        skins
    }

    /// Points the picker's detail pane at the active skin, layout and colour.
    fn sync_dialog(&self) {
        let active = self.active.borrow();
        let Some(active) = active.as_ref() else {
            return;
        };
        self.dialog.set_selected_skin(active.id.as_str().into());
        self.dialog
            .set_selected_layout(active.layout.as_str().into());
        self.dialog
            .set_selected_color_theme(active.color_theme.as_str().into());

        let skins = self.skins.borrow();
        let Some((skin, entry)) = skins.iter().find(|(s, _)| s.manifest.id == active.id) else {
            return;
        };
        self.dialog.set_current(entry.clone());
        let theme = skin
            .manifest
            .color_themes
            .iter()
            .find(|t| t.id == active.color_theme);
        let preview = load_image(skin.preview_path(theme));
        self.dialog.set_has_current_preview(preview.is_some());
        self.dialog.set_current_preview(preview.unwrap_or_default());
    }

    /// Handles a pick in the skin dialog. Empty ids mean "keep what this skin
    /// last used". A colour change on the active layout is applied live; anything
    /// else needs the skin recompiled.
    fn apply_selection(self: &Rc<Self>, id: String, layout: String, color_theme: String) {
        let same_layout = self
            .active
            .borrow()
            .as_ref()
            .is_some_and(|a| a.id == id && (layout.is_empty() || a.layout == layout));
        if same_layout {
            if !color_theme.is_empty() {
                self.apply_color_theme(&color_theme);
            }
            return;
        }
        self.schedule_activate(id, layout, color_theme);
    }

    /// Defers a skin switch to the next event loop iteration, so the old skin
    /// is never destroyed from inside one of its own callbacks.
    fn schedule_activate(self: &Rc<Self>, id: String, layout: String, color_theme: String) {
        let weak = Rc::downgrade(self);
        slint::Timer::single_shot(Duration::ZERO, move || {
            if let Some(app) = weak.upgrade() {
                match app.activate(&id, &layout, &color_theme) {
                    Ok(()) => app.dialog.set_load_error(SharedString::default()),
                    Err(e) => {
                        eprintln!("[skins] {e}");
                        app.dialog.set_load_error(e.into());
                    }
                }
            }
        });
    }

    fn activate(
        self: &Rc<Self>,
        skin_id: &str,
        layout_id: &str,
        color_theme_id: &str,
    ) -> Result<(), String> {
        let skin = self
            .skins
            .borrow()
            .iter()
            .map(|(s, _)| s)
            .find(|s| s.manifest.id == skin_id)
            .cloned()
            .ok_or_else(|| format!("Skin '{skin_id}' is not installed"))?;
        let skin = &skin;

        let mut settings = load_settings();
        let saved = settings.prefs_for(skin_id);
        let pick = |requested: &str, saved: &str| -> Option<String> {
            [requested, saved]
                .into_iter()
                .find(|v| !v.is_empty())
                .map(str::to_string)
        };
        let layout = skin
            .resolve_layout(pick(layout_id, &saved.layout).as_deref())
            .clone();
        let color_theme =
            skin.resolve_color_theme(pick(color_theme_id, &saved.color_theme).as_deref());

        let definition = compile_skin_layout(skin, &layout)?;
        let instance = definition
            .create()
            .map_err(|e| format!("Failed to instantiate skin '{skin_id}': {e:?}"))?;

        self.wire_skin(&instance);
        self.push_state(&instance);
        set_opt(
            &instance,
            "history",
            Value::Model(ModelRc::from(self.history.clone())),
        );
        if !color_theme.is_empty() {
            set_opt(
                &instance,
                "color-theme",
                Value::String(color_theme.as_str().into()),
            );
        }

        instance.window().on_close_requested(|| {
            let _ = slint::quit_event_loop();
            slint::CloseRequestResponse::HideWindow
        });

        // Show the new window before hiding the old one so the app never has zero windows.
        instance
            .show()
            .map_err(|e| format!("Failed to show skin '{skin_id}': {e:?}"))?;
        instance
            .window()
            .set_size(slint::LogicalSize::new(layout.width, layout.height));

        let old = self.active.replace(Some(ActiveSkin {
            id: skin_id.to_string(),
            layout: layout.id.clone(),
            color_theme: color_theme.clone(),
            instance,
        }));
        if let Some(old) = old {
            let _ = old.instance.hide();
        }
        let _ = self.no_skin.hide();

        settings.set_active(
            skin_id,
            SkinPrefs {
                layout: layout.id.clone(),
                color_theme: color_theme.clone(),
            },
        );
        save_settings(&settings);
        self.sync_dialog();
        Ok(())
    }

    /// Switches the active skin's colour theme without recompiling it.
    fn apply_color_theme(&self, color_theme: &str) {
        {
            let mut active = self.active.borrow_mut();
            let Some(active) = active.as_mut() else {
                return;
            };
            let skins = self.skins.borrow();
            let Some((skin, _)) = skins.iter().find(|(s, _)| s.manifest.id == active.id) else {
                return;
            };
            let resolved = skin.resolve_color_theme(Some(color_theme));
            if resolved.is_empty() {
                return;
            }
            set_opt(
                &active.instance,
                "color-theme",
                Value::String(resolved.as_str().into()),
            );
            active.color_theme = resolved.clone();

            let mut settings = load_settings();
            settings.set_active(
                &active.id,
                SkinPrefs {
                    layout: active.layout.clone(),
                    color_theme: resolved,
                },
            );
            save_settings(&settings);
        }
        self.sync_dialog();
    }

    fn wire_skin(self: &Rc<Self>, inst: &ComponentInstance) {
        let on = |name: &str, handler: SkinCallback| {
            let weak: Weak<App> = Rc::downgrade(self);
            let _ = inst.set_callback(name, move |args| {
                if let Some(app) = weak.upgrade() {
                    handler(&app, args);
                }
                Value::Void
            });
        };
        let arg_string = |args: &[Value]| match args.first() {
            Some(Value::String(s)) => s.to_string(),
            _ => String::new(),
        };

        on(
            "press-key",
            Box::new(move |app, args| app.press_key(&arg_string(args))),
        );
        on(
            "expression-edited",
            Box::new(|app, _| app.expression_edited()),
        );
        on(
            "toggle-angle-unit",
            Box::new(|app, _| app.toggle_angle_unit()),
        );
        on(
            "clear-history",
            Box::new(|app, _| {
                app.history.clear();
                clear_saved_history();
            }),
        );
        on(
            "select-history-item",
            Box::new(move |app, args| app.load_expression(arg_string(args))),
        );
        on(
            "set-color-theme",
            Box::new(move |app, args| app.apply_color_theme(&arg_string(args))),
        );
        on(
            "close-window",
            Box::new(|_, _| {
                let _ = slint::quit_event_loop();
            }),
        );
        on(
            "open-skin-dialog",
            Box::new(|app, _| {
                if let Err(e) = show_window(&app.dialog) {
                    eprintln!("Failed to show skin dialog: {e:?}");
                }
            }),
        );

        let weak = Rc::downgrade(self);
        let _ = inst.set_callback("switch-skin", move |args| {
            if let (Some(app), Some(Value::String(id))) = (weak.upgrade(), args.first()) {
                app.schedule_activate(id.to_string(), String::new(), String::new());
            }
            Value::Void
        });
    }

    fn with_instance(&self, f: impl FnOnce(&ComponentInstance)) {
        if let Some(active) = self.active.borrow().as_ref() {
            f(&active.instance);
        }
    }

    /// Reads back edits a skin may have made directly (e.g. typing in a TextInput).
    fn pull_editor(&self, inst: &ComponentInstance) {
        let mut st = self.state.borrow_mut();
        if let Ok(Value::String(expr)) = inst.get_property("expression") {
            st.expression = expr.to_string();
        }
        if let Ok(Value::Number(pos)) = inst.get_property("cursor-byte-offset") {
            st.cursor = pos.max(0.0) as usize;
        } else {
            st.cursor = st.expression.len();
        }
        st.cursor = clamp_char_boundary(&st.expression, st.cursor);
    }

    fn push_state(&self, inst: &ComponentInstance) {
        let st = self.state.borrow();
        set_opt(
            inst,
            "expression",
            Value::String(st.expression.as_str().into()),
        );
        set_opt(inst, "cursor-byte-offset", Value::Number(st.cursor as f64));
        set_opt(
            inst,
            "preview-result",
            Value::String(st.preview.as_str().into()),
        );
        set_opt(inst, "is-error", Value::Bool(st.is_error));
        set_opt(inst, "just-evaluated", Value::Bool(st.just_evaluated));
        set_opt(inst, "use-radians", Value::Bool(st.use_radians));
        let _ = inst.invoke("set-cursor", &[Value::Number(st.cursor as f64)]);
    }

    fn refresh_preview(st: &mut CalcState) {
        st.is_error = false;
        st.preview = live_preview(&st.expression, st.use_radians);
    }

    fn press_key(&self, key: &str) {
        self.with_instance(|inst| {
            self.pull_editor(inst);
            set_opt(inst, "expr-scroll-offset", Value::Number(0.0));
            {
                let mut st = self.state.borrow_mut();
                match apply_key_press(&st.expression, st.cursor, key, st.just_evaluated) {
                    KeyAction::Clear => {
                        st.expression.clear();
                        st.cursor = 0;
                        st.preview.clear();
                        st.is_error = false;
                        st.just_evaluated = false;
                    }
                    KeyAction::Evaluate => {
                        if st.expression.is_empty() {
                            return;
                        }
                        match evaluate_expression(&st.expression, st.use_radians) {
                            Ok(val) => {
                                let formatted = format_number(val);
                                if has_operator_or_func(&st.expression) {
                                    self.history.push(history_value(&st.expression, &formatted));
                                    append_history_entry(&st.expression, &formatted);
                                }
                                st.cursor = formatted.len();
                                st.expression = formatted;
                                st.preview.clear();
                                st.is_error = false;
                                st.just_evaluated = true;
                            }
                            Err(msg) => {
                                if msg.is_empty() {
                                    return;
                                }
                                st.preview = msg;
                                st.is_error = true;
                            }
                        }
                    }
                    KeyAction::UpdateExpression {
                        new_expression,
                        new_cursor,
                    } => {
                        st.just_evaluated = false;
                        st.cursor = clamp_char_boundary(&new_expression, new_cursor);
                        st.expression = new_expression;
                        Self::refresh_preview(&mut st);
                    }
                }
            }
            self.push_state(inst);
        });
    }

    fn expression_edited(&self) {
        self.with_instance(|inst| {
            self.pull_editor(inst);
            let mut st = self.state.borrow_mut();
            st.just_evaluated = false;
            Self::refresh_preview(&mut st);
            // Only push derived values: rewriting `expression` would reset the skin's caret.
            set_opt(
                inst,
                "preview-result",
                Value::String(st.preview.as_str().into()),
            );
            set_opt(inst, "is-error", Value::Bool(st.is_error));
            set_opt(inst, "just-evaluated", Value::Bool(false));
        });
    }

    fn toggle_angle_unit(&self) {
        self.with_instance(|inst| {
            self.pull_editor(inst);
            {
                let mut st = self.state.borrow_mut();
                st.use_radians = !st.use_radians;
                Self::refresh_preview(&mut st);
            }
            self.push_state(inst);
        });
    }

    fn load_expression(&self, expr: String) {
        self.with_instance(|inst| {
            {
                let mut st = self.state.borrow_mut();
                st.cursor = expr.len();
                st.expression = expr;
                st.just_evaluated = false;
                Self::refresh_preview(&mut st);
            }
            self.push_state(inst);
        });
    }
}
