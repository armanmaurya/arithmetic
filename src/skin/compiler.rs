use super::manifest::{SkinInfo, SkinLayout};
use slint_interpreter::{Compiler, ComponentDefinition};
use std::path::Path;

fn compile_file(skin: &SkinInfo, file: &Path) -> Result<ComponentDefinition, String> {
    let mut compiler = Compiler::default();

    // STRICT ISOLATION: The compiler MUST only include the skin's own directory.
    // Skins must be 100% self-contained and not depend on internal application paths.
    compiler.set_include_paths(vec![skin.directory.clone()]);

    let result = spin_on::spin_on(compiler.build_from_path(file));

    let errors: Vec<String> = result
        .diagnostics()
        .filter(|d| d.level() == slint_interpreter::DiagnosticLevel::Error)
        .map(|d| d.to_string())
        .collect();

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

/// Compiles the skin's default layout.
pub fn compile_skin_definition(skin: &SkinInfo) -> Result<ComponentDefinition, String> {
    compile_file(skin, &skin.main_file)
}

/// Compiles a specific layout of the skin.
pub fn compile_skin_layout(skin: &SkinInfo, layout: &SkinLayout) -> Result<ComponentDefinition, String> {
    compile_file(skin, &skin.layout_file(layout))
}
