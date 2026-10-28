use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A layout is a separate `.slint` file with its own window size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinLayout {
    pub id: String,
    pub name: String,
    pub file: String,
    pub width: f32,
    pub height: f32,
}

/// A colour theme is only an id: the skin receives it through its
/// `color-theme` property and owns the matching palette. `swatch` (hex
/// colours) and `preview` (image file) are only used by the skin picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinColorTheme {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub swatch: Vec<String>,
    #[serde(default)]
    pub preview: Option<String>,
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
    /// Screenshot shown in the skin picker, relative to the skin folder.
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default = "default_layout_id")]
    pub default_layout: String,
    pub layouts: Vec<SkinLayout>,
    #[serde(default)]
    pub default_color_theme: String,
    #[serde(default)]
    pub color_themes: Vec<SkinColorTheme>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_layout_id() -> String {
    "standard".to_string()
}

#[derive(Debug, Clone)]
pub struct SkinInfo {
    pub manifest: SkinManifest,
    pub directory: PathBuf,
    pub main_file: PathBuf,
}

impl SkinInfo {
    pub fn get_layout(&self, layout_id: &str) -> Option<&SkinLayout> {
        self.manifest.layouts.iter().find(|l| l.id == layout_id)
    }

    /// The requested layout if it exists, else the default, else the first.
    pub fn resolve_layout<'a>(&'a self, requested: Option<&str>) -> &'a SkinLayout {
        if let Some(req) = requested
            && let Some(l) = self.get_layout(req)
        {
            return l;
        }
        self.get_layout(&self.manifest.default_layout)
            .or_else(|| self.manifest.layouts.first())
            .expect("Skin must have at least one layout")
    }

    /// The requested colour theme if it exists, else the default, else the
    /// first; empty when the skin declares no colour themes.
    pub fn resolve_color_theme(&self, requested: Option<&str>) -> String {
        let themes = &self.manifest.color_themes;
        let exists = |id: &str| themes.iter().any(|t| t.id == id);
        if let Some(req) = requested
            && exists(req)
        {
            return req.to_string();
        }
        if exists(&self.manifest.default_color_theme) {
            return self.manifest.default_color_theme.clone();
        }
        themes.first().map(|t| t.id.clone()).unwrap_or_default()
    }

    pub fn layout_file(&self, layout: &SkinLayout) -> PathBuf {
        self.directory.join(&layout.file)
    }

    /// Picker preview for a colour theme, falling back to the skin's preview.
    pub fn preview_path(&self, color_theme: Option<&SkinColorTheme>) -> Option<PathBuf> {
        color_theme
            .and_then(|t| t.preview.as_ref())
            .or(self.manifest.preview.as_ref())
            .map(|p| self.directory.join(p))
            .filter(|p| p.is_file())
    }
}
