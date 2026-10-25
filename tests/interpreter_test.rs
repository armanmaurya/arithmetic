use arithmetic::engine::{evaluate_expression, format_number, pop_last_token};
use arithmetic::skin::{compile_skin_definition, discover_skins};
use arithmetic::storage::{append_history_entry, clear_saved_history, load_raw_history, load_settings, save_settings, AppSettings};

#[test]
fn test_skin_discovery() {
    let skins = discover_skins();
    let ids: Vec<&str> = skins.iter().map(|s| s.manifest.id.as_str()).collect();

    assert!(ids.contains(&"material-3"), "Must discover material-3 skin");
    assert!(ids.contains(&"classic-desktop"), "Must discover classic-desktop skin");
    assert!(ids.contains(&"casio-fx991ms"), "Must discover casio-fx991ms skin");
    assert!(ids.contains(&"ti-datamath-1977"), "Must discover ti-datamath-1977 skin");
    assert!(ids.contains(&"braun-et66"), "Must discover braun-et66 skin");

    // Ensure compact-bar is now a variant of material-3, not a standalone skin
    assert!(!ids.contains(&"compact-bar"), "compact-bar must be a variant of material-3");
    let m3 = skins.iter().find(|s| s.manifest.id == "material-3").expect("material-3 must exist");
    assert_eq!(m3.manifest.variants.len(), 3, "material-3 must have 3 variants");
    let variant_ids: Vec<&str> = m3.manifest.variants.iter().map(|v| v.id.as_str()).collect();
    assert!(variant_ids.contains(&"compact"), "material-3 must have compact variant");
    assert!(variant_ids.contains(&"standard"), "material-3 must have standard variant");
    assert!(variant_ids.contains(&"large"), "material-3 must have large variant");

    // Ensure removed themes are no longer present
    assert!(!ids.contains(&"casio-scientific"));
    assert!(!ids.contains(&"hp-12c-voyager"));
    assert!(!ids.contains(&"cyberpunk-neon"));
    assert!(!ids.contains(&"skeuomorphic"));
}

#[test]
fn test_external_skin_compilation_outside_repo() {
    // Tests that a user can create a skin in an arbitrary directory outside the repo
    // and it compiles cleanly without any relative dependencies on internal repo files.
    let temp_skin_dir = std::env::temp_dir().join("test_user_external_skin");
    let _ = std::fs::create_dir_all(&temp_skin_dir);

    let manifest_content = r#"{
        "id": "user-custom-skin",
        "name": "User Custom Skin",
        "version": "1.0.0",
        "author": "Community User",
        "description": "Completely isolated custom skin created outside the repo",
        "default_variant": "standard",
        "variants": [
            {
                "id": "standard",
                "name": "Standard",
                "file": "skin.slint",
                "width": 360.0,
                "height": 500.0
            }
        ]
    }"#;
    std::fs::write(temp_skin_dir.join("manifest.json"), manifest_content).unwrap();

    let slint_content = r#"
export struct HistoryEntry {
    expression: string,
    result: string,
    timestamp: string,
}

export component AppWindow inherits Window {
    title: "Community Skin";
    preferred-width: 360px;
    preferred-height: 500px;
    background: #202020;

    in-out property <string> expression: "";
    in-out property <int> cursor-byte-offset: 0;
    in-out property <string> preview-result: "";
    in-out property <bool> is-error: false;
    in-out property <bool> use-radians: false;
    in-out property <[HistoryEntry]> history: [];

    callback press-key(string);
    callback expression-edited();
    callback toggle-angle-unit();
    callback clear-history();
    callback select-history-item(string);
    callback switch-skin(string);
    callback open-skin-dialog();

    Text {
        text: root.expression;
        color: #FFFFFF;
        font-size: 24px;
    }
}
"#;
    std::fs::write(temp_skin_dir.join("skin.slint"), slint_content).unwrap();

    let manifest = serde_json::from_str::<arithmetic::skin::SkinManifest>(manifest_content).unwrap();
    let skin_info = arithmetic::skin::SkinInfo {
        manifest,
        directory: temp_skin_dir.clone(),
        main_file: temp_skin_dir.join("skin.slint"),
    };

    let comp_res = compile_skin_definition(&skin_info);
    assert!(comp_res.is_ok(), "External skin outside repo must compile independently: {:?}", comp_res.err());

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_skin_dir);
}

#[test]
fn test_config_directory_and_skins_path() {
    use arithmetic::storage::{app_config_dir, config_file_path};

    let cfg_dir = app_config_dir().expect("Must resolve config dir");
    let cfg_file = config_file_path().expect("Must resolve config file path");
    assert!(cfg_file.ends_with("config.json"));
    assert!(cfg_file.starts_with(&cfg_dir));

    let skins_dir = cfg_dir.join("skins");
    assert_eq!(skins_dir, cfg_dir.join("skins"));
}

#[test]
fn test_settings_persistence() {
    let original = load_settings();

    save_settings(&AppSettings {
        active_skin: "material-3".to_string(),
        active_variant: "compact".to_string(),
        theme_mode: 1,
    });
    let loaded = load_settings();
    assert_eq!(loaded.active_skin, "material-3");
    assert_eq!(loaded.active_variant, "compact");
    assert_eq!(loaded.theme_mode, 1);

    save_settings(&AppSettings {
        active_skin: "classic-desktop".to_string(),
        active_variant: "standard".to_string(),
        theme_mode: 0,
    });
    let loaded = load_settings();
    assert_eq!(loaded.active_skin, "classic-desktop");
    assert_eq!(loaded.active_variant, "standard");
    assert_eq!(loaded.theme_mode, 0);

    // Restore original
    save_settings(&original);
}

#[test]
fn test_compile_all_skin_definitions() {
    let skins = discover_skins();
    assert!(!skins.is_empty(), "Must discover at least 1 skin");

    for skin in &skins {
        let comp_res = compile_skin_definition(skin);
        assert!(
            comp_res.is_ok(),
            "Skin '{}' failed to compile: {:?}",
            skin.manifest.id,
            comp_res.err()
        );

        let comp = comp_res.unwrap();
        let props: Vec<String> = comp.properties().map(|(name, _)| name).collect();

        // Verify standard skin contract properties
        assert!(
            props.iter().any(|p| p == "expression"),
            "Skin '{}' must have 'expression' property",
            skin.manifest.id
        );
        assert!(
            props.iter().any(|p| p == "preview_result" || p == "preview-result"),
            "Skin '{}' must have 'preview_result' property",
            skin.manifest.id
        );
        assert!(
            props.iter().any(|p| p == "use_radians" || p == "use-radians"),
            "Skin '{}' must have 'use_radians' property",
            skin.manifest.id
        );
    }
}

#[test]
fn test_engine_calculations() {
    // Basic arithmetic
    let res = evaluate_expression("7 + 8", false).unwrap();
    assert_eq!(format_number(res), "15");

    let res = evaluate_expression("12 × 4", false).unwrap();
    assert_eq!(format_number(res), "48");

    let res = evaluate_expression("100 ÷ 4", false).unwrap();
    assert_eq!(format_number(res), "25");

    // Trigonometry (DEG vs RAD)
    let res_deg = evaluate_expression("sin(90)", false).unwrap();
    assert_eq!(format_number(res_deg), "1");

    let res_rad = evaluate_expression("sin(π / 2)", true).unwrap();
    assert_eq!(format_number(res_rad), "1");

    // Token popping (caret backspace)
    let mut expr = "sin(".to_string();
    pop_last_token(&mut expr);
    assert_eq!(expr, "");

    let mut expr = "123".to_string();
    pop_last_token(&mut expr);
    assert_eq!(expr, "12");
}

#[test]
fn test_raw_history_persistence() {
    clear_saved_history();
    append_history_entry("2+2", "4");
    append_history_entry("5*5", "25");

    let history = load_raw_history();
    assert!(history.iter().any(|(e, r)| e == "2+2" && r == "4"));
    assert!(history.iter().any(|(e, r)| e == "5*5" && r == "25"));

    clear_saved_history();
}
