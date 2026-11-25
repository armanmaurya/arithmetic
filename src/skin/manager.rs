use super::compiler::compile_skin_layout;
use super::discovery::{discover_skins, load_skin_from_dir, remote_skins_dir, user_skin_directories};
use super::manifest::{SOURCE_METADATA_FILE, SkinInfo, SkinManifest, SkinSource};
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tar::{Archive, EntryType};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSpec {
    pub owner: String,
    pub repo: String,
}

impl RepoSpec {
    /// Parses `owner/repo` or `https://github.com/owner/repo[.git]`.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err("Repository cannot be empty".to_string());
        }

        let path = if let Some(rest) = trimmed
            .strip_prefix("https://github.com/")
            .or_else(|| trimmed.strip_prefix("http://github.com/"))
            .or_else(|| trimmed.strip_prefix("github.com/"))
        {
            rest
        } else if trimmed.contains("://") {
            return Err(format!(
                "Only GitHub repositories are supported (got '{trimmed}')"
            ));
        } else {
            trimmed
        };

        let path = path.strip_suffix(".git").unwrap_or(path).trim_end_matches('/');
        let mut parts = path.split('/');
        let (Some(owner), Some(repo), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(format!(
                "Expected 'owner/repo' or GitHub repository URL, got '{trimmed}'"
            ));
        };

        validate_slug(owner, "owner")?;
        validate_slug(repo, "repository")?;

        Ok(Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
        })
    }

    pub fn url(&self) -> String {
        format!("https://github.com/{}/{}", self.owner, self.repo)
    }

    pub fn skin_id(&self) -> String {
        format!("{}.{}", self.owner, self.repo)
    }
}

fn validate_slug(part: &str, label: &str) -> Result<(), String> {
    let valid = !part.is_empty()
        && part != "."
        && part != ".."
        && part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !valid {
        return Err(format!("Invalid GitHub {label} name: '{part}'"));
    }
    Ok(())
}

/// Accepts either `alice.retro-calc`, `alice/retro-calc`, or a GitHub URL.
pub fn normalize_skin_target(target: &str) -> String {
    let trimmed = target.trim();
    if let Ok(spec) = RepoSpec::parse(trimmed) {
        spec.skin_id()
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallResult {
    pub skin_id: String,
    pub name: String,
    pub version: String,
    pub commit: String,
    pub updated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Updated(InstallResult),
    UpToDate { skin_id: String, commit: String },
    Pinned { skin_id: String, ref_name: String },
    Failed { skin_id: String, error: String },
}

pub trait SkinFetcher {
    fn resolve_commit(&self, owner: &str, repo: &str, ref_name: Option<&str>) -> Result<String, String>;
    fn download_tarball(
        &self,
        owner: &str,
        repo: &str,
        commit: &str,
        dest_tgz: &Path,
    ) -> Result<(), String>;
}

pub struct CurlFetcher;

impl SkinFetcher for CurlFetcher {
    fn resolve_commit(&self, owner: &str, repo: &str, ref_name: Option<&str>) -> Result<String, String> {
        let target_ref = ref_name.filter(|r| !r.is_empty()).unwrap_or("HEAD");
        let url = format!("https://api.github.com/repos/{owner}/{repo}/commits/{target_ref}");
        let output = Command::new("curl")
            .args([
                "-fsSL",
                "-H",
                &format!("User-Agent: arithmetic/{APP_VERSION}"),
                "-H",
                "Accept: application/vnd.github.sha",
                &url,
            ])
            .output()
            .map_err(|e| curl_exec_error("resolve commit", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Failed to query GitHub for {owner}/{repo} ({target_ref}): {}",
                stderr.trim()
            ));
        }

        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if sha.len() < 7 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "Unexpected commit SHA from GitHub for {owner}/{repo}: '{sha}'"
            ));
        }
        Ok(sha)
    }

    fn download_tarball(
        &self,
        owner: &str,
        repo: &str,
        commit: &str,
        dest_tgz: &Path,
    ) -> Result<(), String> {
        let url = format!("https://github.com/{owner}/{repo}/archive/{commit}.tar.gz");
        let output = Command::new("curl")
            .args([
                "-fsSL",
                "-H",
                &format!("User-Agent: arithmetic/{APP_VERSION}"),
                "-o",
            ])
            .arg(dest_tgz)
            .arg(&url)
            .output()
            .map_err(|e| curl_exec_error("download tarball", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Failed to download {owner}/{repo} archive: {}",
                stderr.trim()
            ));
        }
        Ok(())
    }
}

fn curl_exec_error(action: &str, err: std::io::Error) -> String {
    if err.kind() == ErrorKind::NotFound {
        "The 'curl' command is required to install or update remote skins".to_string()
    } else {
        format!("Failed to {action} with curl: {err}")
    }
}

pub fn install_remote_skin(repo_spec: &str, ref_name: Option<&str>) -> Result<InstallResult, String> {
    let dest_root = remote_skins_dir()
        .ok_or_else(|| "Could not determine a writable skins data directory".to_string())?;
    install_remote_skin_with(&CurlFetcher, &dest_root, repo_spec, ref_name)
}

pub fn update_remote_skins(target: Option<&str>, force: bool) -> Result<Vec<UpdateStatus>, String> {
    let dest_root = remote_skins_dir()
        .ok_or_else(|| "Could not determine a writable skins data directory".to_string())?;
    let skins = discover_skins();
    update_remote_skins_with(&CurlFetcher, &dest_root, &skins, target, force)
}

pub fn remove_skin(target: &str) -> Result<String, String> {
    let skins = discover_skins();
    let user_dirs = user_skin_directories();
    remove_skin_from(&skins, &user_dirs, target)
}

pub fn install_remote_skin_with(
    fetcher: &dyn SkinFetcher,
    dest_root: &Path,
    repo_spec: &str,
    ref_name: Option<&str>,
) -> Result<InstallResult, String> {
    let spec = RepoSpec::parse(repo_spec)?;
    let pinned = ref_name.is_some_and(|r| !r.trim().is_empty());
    let ref_label = ref_name
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .unwrap_or("HEAD")
        .to_string();

    let commit = fetcher.resolve_commit(&spec.owner, &spec.repo, Some(&ref_label))?;
    let target_dir = dest_root.join(spec.skin_id());
    let updated = target_dir.is_dir();

    stage_validate_and_swap(
        fetcher,
        dest_root,
        &target_dir,
        &spec,
        &ref_label,
        pinned,
        &commit,
        updated,
    )
}

pub fn update_remote_skins_with(
    fetcher: &dyn SkinFetcher,
    dest_root: &Path,
    skins: &[SkinInfo],
    target: Option<&str>,
    force: bool,
) -> Result<Vec<UpdateStatus>, String> {
    let normalized_target = target.map(normalize_skin_target);
    let mut remote_skins: Vec<&SkinInfo> = skins.iter().filter(|s| s.source.is_some()).collect();

    if let Some(ref id) = normalized_target {
        remote_skins.retain(|s| s.manifest.id == *id);
        if remote_skins.is_empty() {
            return Err(format!("No GitHub-installed skin found matching '{id}'"));
        }
    }

    let mut statuses = Vec::new();
    for skin in remote_skins {
        let Some(src) = skin.source.as_ref() else {
            continue;
        };
        let skin_id = src.skin_id();
        if src.pinned && !force {
            statuses.push(UpdateStatus::Pinned {
                skin_id,
                ref_name: src.ref_name.clone(),
            });
            continue;
        }

        let commit = match fetcher.resolve_commit(&src.owner, &src.repo, Some(&src.ref_name)) {
            Ok(sha) => sha,
            Err(error) => {
                statuses.push(UpdateStatus::Failed { skin_id, error });
                continue;
            }
        };

        if commit == src.commit && !force {
            statuses.push(UpdateStatus::UpToDate { skin_id, commit });
            continue;
        }

        let spec = RepoSpec {
            owner: src.owner.clone(),
            repo: src.repo.clone(),
        };
        match stage_validate_and_swap(
            fetcher,
            dest_root,
            &skin.directory,
            &spec,
            &src.ref_name,
            src.pinned,
            &commit,
            true,
        ) {
            Ok(result) => statuses.push(UpdateStatus::Updated(result)),
            Err(error) => statuses.push(UpdateStatus::Failed { skin_id, error }),
        }
    }

    Ok(statuses)
}

pub fn remove_skin_from(
    skins: &[SkinInfo],
    user_dirs: &[PathBuf],
    target: &str,
) -> Result<String, String> {
    let id = normalize_skin_target(target);
    let Some(skin) = skins.iter().find(|s| s.manifest.id == id) else {
        return Err(format!("Skin '{id}' is not installed"));
    };

    if !is_inside_any(&skin.directory, user_dirs) {
        return Err(format!(
            "Skin '{id}' is a bundled skin at {:?} and cannot be removed here",
            skin.directory
        ));
    }

    fs::remove_dir_all(&skin.directory)
        .map_err(|e| format!("Failed to remove skin '{id}': {e}"))?;
    Ok(id)
}

pub fn is_inside_any(path: &Path, roots: &[PathBuf]) -> bool {
    let Ok(canon_path) = path.canonicalize() else {
        return false;
    };
    roots.iter().any(|root| {
        root.canonicalize()
            .is_ok_and(|canon_root| canon_path.starts_with(canon_root))
    })
}

#[allow(clippy::too_many_arguments)]
fn stage_validate_and_swap(
    fetcher: &dyn SkinFetcher,
    dest_root: &Path,
    target_dir: &Path,
    spec: &RepoSpec,
    ref_name: &str,
    pinned: bool,
    commit: &str,
    updated: bool,
) -> Result<InstallResult, String> {
    fs::create_dir_all(dest_root)
        .map_err(|e| format!("Failed to create skins directory {:?}: {e}", dest_root))?;

    let stamp = unique_stamp();
    let tgz_path = dest_root.join(format!(".tmp-{}-{stamp}.tar.gz", spec.skin_id()));
    let staging_dir = dest_root.join(format!(".tmp-{}-{stamp}", spec.skin_id()));
    let _cleanup = TempCleanup {
        tgz: tgz_path.clone(),
        dir: staging_dir.clone(),
    };

    fetcher.download_tarball(&spec.owner, &spec.repo, commit, &tgz_path)?;
    extract_github_tarball(&tgz_path, &staging_dir)?;

    let manifest = read_and_validate_manifest(&staging_dir)?;
    let source = SkinSource {
        owner: spec.owner.clone(),
        repo: spec.repo.clone(),
        url: spec.url(),
        ref_name: ref_name.to_string(),
        pinned,
        commit: commit.to_string(),
        version: manifest.version.clone(),
    };
    let source_json = serde_json::to_string_pretty(&source)
        .map_err(|e| format!("Failed to serialize source metadata: {e}"))?;
    fs::write(staging_dir.join(SOURCE_METADATA_FILE), source_json)
        .map_err(|e| format!("Failed to write {SOURCE_METADATA_FILE}: {e}"))?;

    let skin_info = load_skin_from_dir(&staging_dir)?;
    for layout in &skin_info.manifest.layouts {
        compile_skin_layout(&skin_info, layout)?;
    }

    swap_directory(&staging_dir, target_dir)?;

    Ok(InstallResult {
        skin_id: spec.skin_id(),
        name: skin_info.manifest.name,
        version: skin_info.manifest.version,
        commit: commit.to_string(),
        updated,
    })
}

fn read_and_validate_manifest(staging_dir: &Path) -> Result<SkinManifest, String> {
    let manifest_path = staging_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(
            "Repository root does not contain a 'manifest.json' file".to_string(),
        );
    }
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest.json: {e}"))?;
    let manifest: SkinManifest = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid manifest.json: {e}"))?;

    if manifest.layouts.is_empty() {
        return Err("Skin manifest.json defines no layouts".to_string());
    }

    if let Some(min_ver) = &manifest.min_arithmetic {
        check_min_arithmetic(min_ver, APP_VERSION)?;
    }

    let canon_root = staging_dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve staging directory: {e}"))?;

    for layout in &manifest.layouts {
        validate_relative_file(&canon_root, &layout.file, true)?;
    }
    if let Some(preview) = &manifest.preview {
        validate_relative_file(&canon_root, preview, false)?;
    }
    for theme in &manifest.color_themes {
        if let Some(preview) = &theme.preview {
            validate_relative_file(&canon_root, preview, false)?;
        }
    }

    Ok(manifest)
}

fn validate_relative_file(canon_root: &Path, rel: &str, must_exist: bool) -> Result<(), String> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute()
        || rel_path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("Unsafe skin asset path '{rel}'"));
    }

    let joined = canon_root.join(rel_path);
    if must_exist && !joined.is_file() {
        return Err(format!("Missing layout file '{rel}' referenced in manifest.json"));
    }
    if joined.exists() {
        let canon = joined
            .canonicalize()
            .map_err(|e| format!("Invalid path '{rel}': {e}"))?;
        if !canon.starts_with(canon_root) {
            return Err(format!("Path '{rel}' escapes skin directory"));
        }
    }
    Ok(())
}

pub fn check_min_arithmetic(min_version: &str, current_version: &str) -> Result<(), String> {
    let min = parse_semver(min_version)
        .ok_or_else(|| format!("Invalid min_arithmetic version '{min_version}'"))?;
    let current = parse_semver(current_version)
        .ok_or_else(|| format!("Invalid current version '{current_version}'"))?;
    if current < min {
        return Err(format!(
            "Skin requires Arithmetic >= {min_version}, but running {current_version}"
        ));
    }
    Ok(())
}

fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let cleaned = raw.trim().strip_prefix('v').unwrap_or(raw.trim());
    let core = cleaned.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn extract_github_tarball(tgz_path: &Path, dest_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dest_dir)
        .map_err(|e| format!("Failed to create staging dir {:?}: {e}", dest_dir))?;

    let file = File::open(tgz_path)
        .map_err(|e| format!("Failed to open archive {:?}: {e}", tgz_path))?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    let entries = archive
        .entries()
        .map_err(|e| format!("Failed to read archive: {e}"))?;

    let mut top_root: Option<String> = None;

    for entry_res in entries {
        let mut entry = entry_res.map_err(|e| format!("Corrupt archive entry: {e}"))?;
        let entry_type = entry.header().entry_type();

        // GitHub archives begin with a PAX global header containing the commit SHA.
        if matches!(entry_type, EntryType::XGlobalHeader | EntryType::XHeader) {
            continue;
        }
        if !matches!(entry_type, EntryType::Regular | EntryType::Directory) {
            return Err(format!(
                "Archive contains unsupported entry type ({entry_type:?})"
            ));
        }

        let entry_path = entry
            .path()
            .map_err(|e| format!("Invalid path in archive: {e}"))?
            .into_owned();

        let mut components = entry_path.components();
        let Some(Component::Normal(first)) = components.next() else {
            return Err(format!("Unsafe archive path: {:?}", entry_path));
        };
        let first_str = first.to_string_lossy();
        match &top_root {
            Some(root) if root != first_str.as_ref() => {
                return Err("Archive has unexpected multiple root directories".to_string());
            }
            None => top_root = Some(first_str.into_owned()),
            _ => {}
        }

        let mut rel = PathBuf::new();
        for comp in components {
            match comp {
                Component::Normal(seg) => rel.push(seg),
                _ => return Err(format!("Unsafe path component in archive: {:?}", entry_path)),
            }
        }
        if rel.as_os_str().is_empty() {
            continue;
        }

        let out_path = dest_dir.join(&rel);
        if entry_type == EntryType::Directory {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create directory {:?}: {e}", out_path))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory {:?}: {e}", parent))?;
            }
            let mut out_file = File::create(&out_path)
                .map_err(|e| format!("Failed to create file {:?}: {e}", out_path))?;
            std::io::copy(&mut entry, &mut out_file)
                .map_err(|e| format!("Failed to extract file {:?}: {e}", out_path))?;
        }
    }

    Ok(())
}

fn swap_directory(staging_dir: &Path, target_dir: &Path) -> Result<(), String> {
    if !target_dir.exists() {
        return fs::rename(staging_dir, target_dir)
            .map_err(|e| format!("Failed to install skin to {:?}: {e}", target_dir));
    }

    let parent = target_dir
        .parent()
        .ok_or_else(|| "Invalid target directory".to_string())?;
    let name = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("skin");
    let backup_dir = parent.join(format!(".bak-{name}-{}", unique_stamp()));

    fs::rename(target_dir, &backup_dir)
        .map_err(|e| format!("Failed to back up existing skin {:?}: {e}", target_dir))?;

    if let Err(err) = fs::rename(staging_dir, target_dir) {
        let _ = fs::rename(&backup_dir, target_dir);
        return Err(format!("Failed to replace skin {:?}: {err}", target_dir));
    }

    let _ = fs::remove_dir_all(&backup_dir);
    Ok(())
}

fn unique_stamp() -> u128 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    nanos ^ (std::process::id() as u128)
}

struct TempCleanup {
    tgz: PathBuf,
    dir: PathBuf,
}

impl Drop for TempCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.tgz);
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::collections::HashMap;
    use tar::{Builder, Header};

    struct MockFetcher {
        commits: HashMap<(String, String), String>,
        archives: HashMap<String, Vec<u8>>,
    }

    impl SkinFetcher for MockFetcher {
        fn resolve_commit(
            &self,
            owner: &str,
            repo: &str,
            _ref_name: Option<&str>,
        ) -> Result<String, String> {
            self.commits
                .get(&(owner.to_string(), repo.to_string()))
                .cloned()
                .ok_or_else(|| format!("not found: {owner}/{repo}"))
        }

        fn download_tarball(
            &self,
            _owner: &str,
            _repo: &str,
            commit: &str,
            dest_tgz: &Path,
        ) -> Result<(), String> {
            let bytes = self
                .archives
                .get(commit)
                .ok_or_else(|| format!("missing archive for {commit}"))?;
            fs::write(dest_tgz, bytes).map_err(|e| e.to_string())
        }
    }

    fn valid_slint(label: &str) -> String {
        format!(
            r#"
export component AppWindow inherits Window {{
    in-out property <string> expression: "";
    in-out property <string> preview-result: "";
    in-out property <bool> is-error: false;
    in-out property <bool> use-radians: false;
    callback press-key(string);
    callback open-skin-dialog();
    Text {{ text: "{label}"; }}
}}
"#
        )
    }

    fn valid_manifest(id: &str, name: &str, version: &str, min_arithmetic: Option<&str>) -> String {
        let min_field = min_arithmetic
            .map(|v| format!(r#""min_arithmetic": "{v}","#))
            .unwrap_or_default();
        format!(
            r#"{{
    "id": "{id}",
    "name": "{name}",
    "version": "{version}",
    {min_field}
    "default_layout": "standard",
    "layouts": [
        {{ "id": "standard", "name": "Standard", "file": "skin.slint", "width": 320.0, "height": 480.0 }}
    ]
}}"#
        )
    }

    fn build_tar_gz(root: &str, files: &[(&str, &str)], extra_symlink: Option<&str>) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let encoder = GzEncoder::new(&mut buf, Compression::default());
            let mut builder = Builder::new(encoder);

            // Include a PAX global header like GitHub archives do.
            let comment = b"52 comment=deadbeef\n";
            let mut pax = Header::new_ustar();
            pax.set_entry_type(EntryType::XGlobalHeader);
            pax.set_size(comment.len() as u64);
            pax.set_cksum();
            builder.append_data(&mut pax, "pax_global_header", &comment[..]).unwrap();

            for (rel, body) in files {
                let path = format!("{root}/{rel}");
                let bytes = body.as_bytes();
                let mut header = Header::new_gnu();
                header.set_size(bytes.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                builder.append_data(&mut header, path, bytes).unwrap();
            }

            if let Some(link_path) = extra_symlink {
                let mut header = Header::new_gnu();
                header.set_entry_type(EntryType::Symlink);
                header.set_size(0);
                header.set_mode(0o777);
                header.set_link_name("/etc/passwd").unwrap();
                header.set_cksum();
                builder
                    .append_data(&mut header, format!("{root}/{link_path}"), &[][..])
                    .unwrap();
            }

            builder.into_inner().unwrap().finish().unwrap();
        }
        buf
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("arithmetic_mgr_{tag}_{}", unique_stamp()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parse_repo_specs_and_targets() {
        let a = RepoSpec::parse("alice/retro-calc").unwrap();
        assert_eq!(a.owner, "alice");
        assert_eq!(a.repo, "retro-calc");
        assert_eq!(a.skin_id(), "alice.retro-calc");

        let b = RepoSpec::parse("https://github.com/alice/retro-calc.git/").unwrap();
        assert_eq!(b, a);

        let c = RepoSpec::parse("github.com/Bob_1/My.Skin").unwrap();
        assert_eq!(c.skin_id(), "Bob_1.My.Skin");

        assert!(RepoSpec::parse("https://gitlab.com/alice/retro").is_err());
        assert!(RepoSpec::parse("../evil/repo").is_err());
        assert!(RepoSpec::parse("onlyone").is_err());

        assert_eq!(normalize_skin_target("alice/retro-calc"), "alice.retro-calc");
        assert_eq!(normalize_skin_target("alice.retro-calc"), "alice.retro-calc");
    }

    #[test]
    fn semver_check_enforces_min_arithmetic() {
        assert!(check_min_arithmetic("0.3.0", "0.3.0").is_ok());
        assert!(check_min_arithmetic("0.2", "0.3.0").is_ok());
        assert!(check_min_arithmetic("v0.3.0", "0.3.1").is_ok());
        assert!(check_min_arithmetic("0.4.0", "0.3.0").is_err());
        assert!(check_min_arithmetic("1.0.0", "0.9.9").is_err());
    }

    #[test]
    fn two_authors_same_manifest_id_do_not_collide() {
        let root = temp_dir("collision");
        let sha_a = "1111111111111111111111111111111111111111".to_string();
        let sha_b = "2222222222222222222222222222222222222222".to_string();

        let fetcher = MockFetcher {
            commits: HashMap::from([
                (("alice".into(), "retro".into()), sha_a.clone()),
                (("bob".into(), "retro".into()), sha_b.clone()),
            ]),
            archives: HashMap::from([
                (
                    sha_a.clone(),
                    build_tar_gz(
                        &format!("retro-{sha_a}"),
                        &[
                            ("manifest.json", &valid_manifest("retro", "Alice Retro", "1.0.0", None)),
                            ("skin.slint", &valid_slint("Alice")),
                        ],
                        None,
                    ),
                ),
                (
                    sha_b.clone(),
                    build_tar_gz(
                        &format!("retro-{sha_b}"),
                        &[
                            ("manifest.json", &valid_manifest("retro", "Bob Retro", "1.0.0", None)),
                            ("skin.slint", &valid_slint("Bob")),
                        ],
                        None,
                    ),
                ),
            ]),
        };

        let res_a = install_remote_skin_with(&fetcher, &root, "alice/retro", None).unwrap();
        let res_b = install_remote_skin_with(&fetcher, &root, "bob/retro", None).unwrap();
        assert_eq!(res_a.skin_id, "alice.retro");
        assert_eq!(res_b.skin_id, "bob.retro");

        let skin_a = load_skin_from_dir(&root.join("alice.retro")).unwrap();
        let skin_b = load_skin_from_dir(&root.join("bob.retro")).unwrap();
        assert_eq!(skin_a.manifest.id, "alice.retro");
        assert_eq!(skin_b.manifest.id, "bob.retro");
        assert_eq!(skin_a.manifest.name, "Alice Retro");
        assert_eq!(skin_b.manifest.name, "Bob Retro");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn update_and_rollback_on_broken_skin() {
        let root = temp_dir("update");
        let sha1 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
        let sha2 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();
        let sha3 = "cccccccccccccccccccccccccccccccccccccccc".to_string();

        let mut fetcher = MockFetcher {
            commits: HashMap::from([(("alice".into(), "calc".into()), sha1.clone())]),
            archives: HashMap::from([
                (
                    sha1.clone(),
                    build_tar_gz(
                        &format!("calc-{sha1}"),
                        &[
                            ("manifest.json", &valid_manifest("calc", "Calc", "1.0.0", None)),
                            ("skin.slint", &valid_slint("v1")),
                        ],
                        None,
                    ),
                ),
                (
                    sha2.clone(),
                    build_tar_gz(
                        &format!("calc-{sha2}"),
                        &[
                            ("manifest.json", &valid_manifest("calc", "Calc", "1.1.0", None)),
                            ("skin.slint", "broken slint syntax {{{"),
                        ],
                        None,
                    ),
                ),
                (
                    sha3.clone(),
                    build_tar_gz(
                        &format!("calc-{sha3}"),
                        &[
                            ("manifest.json", &valid_manifest("calc", "Calc", "1.2.0", None)),
                            ("skin.slint", &valid_slint("v2")),
                        ],
                        None,
                    ),
                ),
            ]),
        };

        install_remote_skin_with(&fetcher, &root, "alice/calc", None).unwrap();
        let installed = vec![load_skin_from_dir(&root.join("alice.calc")).unwrap()];

        // Same commit -> UpToDate
        let statuses = update_remote_skins_with(&fetcher, &root, &installed, None, false).unwrap();
        assert_eq!(
            statuses,
            vec![UpdateStatus::UpToDate {
                skin_id: "alice.calc".into(),
                commit: sha1.clone(),
            }]
        );

        // Broken update -> Failed, old v1.0.0 stays intact
        fetcher
            .commits
            .insert(("alice".into(), "calc".into()), sha2.clone());
        let statuses = update_remote_skins_with(&fetcher, &root, &installed, None, false).unwrap();
        assert!(matches!(statuses.as_slice(), [UpdateStatus::Failed { .. }]));
        let still_v1 = load_skin_from_dir(&root.join("alice.calc")).unwrap();
        assert_eq!(still_v1.manifest.version, "1.0.0");

        // Valid update -> Updated to 1.2.0
        fetcher
            .commits
            .insert(("alice".into(), "calc".into()), sha3.clone());
        let statuses = update_remote_skins_with(&fetcher, &root, &installed, None, false).unwrap();
        assert!(matches!(
            statuses.as_slice(),
            [UpdateStatus::Updated(InstallResult { version, .. })] if version == "1.2.0"
        ));
        let now_v2 = load_skin_from_dir(&root.join("alice.calc")).unwrap();
        assert_eq!(now_v2.manifest.version, "1.2.0");

        // Remove works for user dir, refuses outside user dir
        let removed =
            remove_skin_from(std::slice::from_ref(&now_v2), std::slice::from_ref(&root), "alice/calc")
                .unwrap();
        assert_eq!(removed, "alice.calc");
        assert!(!root.join("alice.calc").exists());
        assert!(remove_skin_from(&[now_v2], &[], "alice.calc").is_err());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_symlinks_and_too_new_min_arithmetic() {
        let root = temp_dir("safety");
        let sha_sym = "dddddddddddddddddddddddddddddddddddddddd".to_string();
        let sha_ver = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string();

        let fetcher = MockFetcher {
            commits: HashMap::from([
                (("alice".into(), "sym".into()), sha_sym.clone()),
                (("alice".into(), "future".into()), sha_ver.clone()),
            ]),
            archives: HashMap::from([
                (
                    sha_sym.clone(),
                    build_tar_gz(
                        &format!("sym-{sha_sym}"),
                        &[
                            ("manifest.json", &valid_manifest("sym", "Sym", "1.0.0", None)),
                            ("skin.slint", &valid_slint("ok")),
                        ],
                        Some("leak"),
                    ),
                ),
                (
                    sha_ver.clone(),
                    build_tar_gz(
                        &format!("future-{sha_ver}"),
                        &[
                            ("manifest.json", &valid_manifest("future", "Future", "1.0.0", Some("99.0.0"))),
                            ("skin.slint", &valid_slint("future")),
                        ],
                        None,
                    ),
                ),
            ]),
        };

        let sym_err = install_remote_skin_with(&fetcher, &root, "alice/sym", None).unwrap_err();
        assert!(sym_err.contains("unsupported entry type"));
        assert!(!root.join("alice.sym").exists());

        let ver_err = install_remote_skin_with(&fetcher, &root, "alice/future", None).unwrap_err();
        assert!(ver_err.contains("Arithmetic >="));
        assert!(!root.join("alice.future").exists());

        let _ = fs::remove_dir_all(&root);
    }
}
