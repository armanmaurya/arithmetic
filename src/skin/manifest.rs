use serde::{Deserialize, Serialize};
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
        if let Some(req) = requested
            && let Some(v) = self.get_variant(req)
        {
            return v;
        }
        self.get_variant(&self.manifest.default_variant)
            .or_else(|| self.manifest.variants.first())
            .expect("Skin must have at least one variant")
    }

    pub fn variant_file(&self, variant: &SkinVariant) -> PathBuf {
        self.directory.join(&variant.file)
    }
}
