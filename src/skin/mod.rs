pub mod compiler;
pub mod discovery;
pub mod manager;
pub mod manifest;

pub use compiler::{compile_skin_definition, compile_skin_layout};
pub use discovery::{
    discover_skins, load_skin_from_dir, remote_skins_dir, skin_search_directories,
    user_skin_directories,
};
pub use manager::{
    InstallResult, RepoSpec, UpdateStatus, install_remote_skin, normalize_skin_target, remove_skin,
    update_remote_skins,
};
pub use manifest::{
    SOURCE_METADATA_FILE, SkinColorTheme, SkinInfo, SkinLayout, SkinManifest, SkinSource,
};
