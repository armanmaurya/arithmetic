use super::discovery::discover_skins;
use super::manifest::SkinInfo;
use slint_interpreter::{Compiler, ComponentDefinition, ComponentInstance};

pub fn compile_skin_definition(skin: &SkinInfo) -> Result<ComponentDefinition, String> {
    let mut compiler = Compiler::default();

    // STRICT ISOLATION: The compiler MUST only include the skin's own directory.
    // Skins must be 100% self-contained and not depend on internal application paths.
    let include_paths = vec![skin.directory.clone()];
    compiler.set_include_paths(include_paths);

    let result = spin_on::spin_on(compiler.build_from_path(&skin.main_file));

    let mut errors = Vec::new();
    for diag in result.diagnostics() {
        if diag.level() == slint_interpreter::DiagnosticLevel::Error {
            errors.push(diag.to_string());
        }
    }

    if !errors.is_empty() {
        return Err(format!(
            "Compilation errors in skin '{}':\n{}",
            skin.manifest.id,
            errors.join("\n")
        ));
    }

    result
        .component("AppWindow")
        .ok_or_else(|| format!("Skin '{}' does not export an 'AppWindow' component", skin.manifest.id))
}

pub fn compile_skin(skin: &SkinInfo) -> Result<ComponentInstance, String> {
    let comp = compile_skin_definition(skin)?;
    comp.create()
        .map_err(|e| format!("Failed to instantiate skin '{}': {:?}", skin.manifest.id, e))
}

pub fn load_skin(id: &str) -> (SkinInfo, ComponentInstance) {
    let skins = discover_skins();

    // 1. Try requested skin
    if let Some(skin) = skins.iter().find(|s| s.manifest.id == id)
        && let Ok(inst) = compile_skin(skin)
    {
        return (skin.clone(), inst);
    }

    // 2. Try default 'material-3' skin
    if id != "material-3"
        && let Some(m3) = skins.iter().find(|s| s.manifest.id == "material-3")
        && let Ok(inst) = compile_skin(m3)
    {
        return (m3.clone(), inst);
    }

    // 3. Try any available skin
    for skin in &skins {
        if let Ok(inst) = compile_skin(skin) {
            return (skin.clone(), inst);
        }
    }

    panic!("Fatal: No valid skin found to render the calculator!");
}
