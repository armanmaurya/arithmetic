use serde::{Deserialize, Serialize};
use slint_interpreter::{Compiler, ComponentDefinition, ComponentInstance};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinVariant {
    pub id: String,
    pub name: String,
    pub file: String,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinManifest {
    pub id: String,
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_variant_id")]
    pub default_variant: String,
    pub variants: Vec<SkinVariant>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_variant_id() -> String {
    "standard".to_string()
}

#[derive(Debug, Clone)]
pub struct SkinInfo {
    pub manifest: SkinManifest,
    pub directory: PathBuf,
    pub main_file: PathBuf,
}

impl SkinInfo {
    pub fn get_variant(&self, variant_id: &str) -> Option<&SkinVariant> {
        self.manifest.variants.iter().find(|v| v.id == variant_id)
    }

    pub fn active_variant<'a>(&'a self, requested: Option<&str>) -> &'a SkinVariant {
        if let Some(req) = requested {
            if let Some(v) = self.get_variant(req) {
                return v;
            }
        }
        self.get_variant(&self.manifest.default_variant)
            .or_else(|| self.manifest.variants.first())
            .expect("Skin must have at least one variant")
    }

    pub fn variant_file(&self, variant: &SkinVariant) -> PathBuf {
        self.directory.join(&variant.file)
    }
}

/// Returns standard search directories for skins
pub fn skin_search_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. User config directory: $XDG_CONFIG_HOME/arithmetic/skins or ~/.config/arithmetic/skins
    if let Some(cfg_dir) = crate::storage::app_config_dir() {
        dirs.push(cfg_dir.join("skins"));
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        dirs.push(PathBuf::from(xdg).join("arithmetic").join("skins"));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".config").join("arithmetic").join("skins"));
    }

    // 2. User data directory: $XDG_DATA_HOME/arithmetic/skins or ~/.local/share/arithmetic/skins
    if let Some(data_dir) = crate::storage::app_data_dir() {
        dirs.push(data_dir.join("skins"));
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(xdg).join("arithmetic").join("skins"));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local").join("share").join("arithmetic").join("skins"));
    }

    // 4. Executable directory: <exe_dir>/skins and <exe_dir>/../share/arithmetic/skins
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("skins"));
            dirs.push(parent.join("..").join("share").join("arithmetic").join("skins"));
        }
    }

    // 5. System directories: /usr/share/arithmetic/skins and /usr/local/share/arithmetic/skins
    dirs.push(PathBuf::from("/usr/share/arithmetic/skins"));
    dirs.push(PathBuf::from("/usr/local/share/arithmetic/skins"));

    // 6. In test or development mode (ARITHMETIC_DEV=1 or cargo test), also check ./skins
    #[cfg(test)]
    {
        if let Ok(cwd) = std::env::current_dir() {
            dirs.push(cwd.join("skins"));
        }
    }
    #[cfg(not(test))]
    {
        if std::env::var_os("ARITHMETIC_DEV").is_some() {
            if let Ok(cwd) = std::env::current_dir() {
                dirs.push(cwd.join("skins"));
            }
        }
    }

    dirs
}

/// Discovers all available skins from the filesystem
pub fn discover_skins() -> Vec<SkinInfo> {
    let mut discovered = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for dir in skin_search_directories() {
        if !dir.is_dir() {
            continue;
        }

        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let skin_dir = entry.path();
            if !skin_dir.is_dir() {
                continue;
            }

            let manifest_path = skin_dir.join("manifest.json");
            if !manifest_path.is_file() {
                continue;
            }

            let Ok(content) = fs::read_to_string(&manifest_path) else {
                continue;
            };

            let Ok(manifest) = serde_json::from_str::<SkinManifest>(&content) else {
                eprintln!("[skins] Invalid manifest at {:?}", manifest_path);
                continue;
            };

            if manifest.variants.is_empty() {
                eprintln!("[skins] Manifest at {:?} has no variants", manifest_path);
                continue;
            }

            if seen_ids.contains(&manifest.id) {
                continue;
            }

            let def_variant = manifest
                .variants
                .iter()
                .find(|v| v.id == manifest.default_variant)
                .unwrap_or(&manifest.variants[0]);
            let main_file = skin_dir.join(&def_variant.file);
            if !main_file.is_file() {
                eprintln!("[skins] Default variant file missing for skin {}: {:?}", manifest.id, main_file);
                continue;
            }

            seen_ids.insert(manifest.id.clone());
            discovered.push(SkinInfo {
                manifest,
                directory: skin_dir,
                main_file,
            });
        }
    }

    discovered
}

/// Compiles a skin into its ComponentDefinition without instantiating a window
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

/// Compiles a skin from its SkinInfo into a ComponentInstance
pub fn compile_skin(skin: &SkinInfo) -> Result<ComponentInstance, String> {
    let comp = compile_skin_definition(skin)?;
    comp.create()
        .map_err(|e| format!("Failed to instantiate skin '{}': {:?}", skin.manifest.id, e))
}

/// Loads a skin by its ID, with automatic fallback
pub fn load_skin(id: &str) -> (SkinInfo, ComponentInstance) {
    let skins = discover_skins();

    // 1. Try requested skin
    if let Some(skin) = skins.iter().find(|s| s.manifest.id == id) {
        match compile_skin(skin) {
            Ok(inst) => return (skin.clone(), inst),
            Err(e) => {
                eprintln!("[skins] Error loading skin '{}': {}. Falling back...", id, e);
            }
        }
    }

    // 2. Try default 'material-3' skin
    if id != "material-3" {
        if let Some(m3) = skins.iter().find(|s| s.manifest.id == "material-3") {
            if let Ok(inst) = compile_skin(m3) {
                return (m3.clone(), inst);
            }
        }
    }

    // 3. Try any available skin
    for skin in &skins {
        if let Ok(inst) = compile_skin(skin) {
            return (skin.clone(), inst);
        }
    }

    panic!("Fatal: No valid skin found to render the calculator!");
}
