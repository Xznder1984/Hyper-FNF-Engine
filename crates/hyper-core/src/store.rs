use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::archive::{find_engine_root, safe_extract_zip};
use crate::engine::EngineDef;
use crate::error::{HyperError, Result};
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersion {
    pub version: String,
    pub dir: PathBuf,
    pub installed_at: u64,
    #[serde(default)]
    pub downloaded_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EngineState {
    pub id: String,
    #[serde(default)]
    pub versions: Vec<InstalledVersion>,
}

impl EngineState {
    fn state_path(id: &str) -> PathBuf {
        paths::engines_dir().join(format!("{id}.json"))
    }

    pub fn load(id: &str) -> Result<Self> {
        let p = Self::state_path(id);
        if !p.exists() {
            return Ok(EngineState {
                id: id.to_string(),
                versions: vec![],
            });
        }
        let text = fs::read_to_string(&p)?;
        serde_json::from_str(&text).map_err(HyperError::from)
    }

    pub fn save(&self) -> Result<()> {
        let p = Self::state_path(&self.id);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&p, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    pub fn installed(&self, version: &str) -> bool {
        self.versions.iter().any(|v| v.version == version)
    }

    pub fn dir_for(&self, version: &str) -> Option<PathBuf> {
        self.versions
            .iter()
            .find(|v| v.version == version)
            .map(|v| v.dir.clone())
    }

    /// Newest installed version directory ("latest"), numeric-aware.
    pub fn latest_dir(&self) -> Option<PathBuf> {
        self.versions.iter().fold(None, |best, v| match best {
            None => Some(v),
            Some(b) if cmp_versions(&v.version, &b.version).is_gt() => Some(v),
            Some(b) => Some(b),
        })
        .map(|v| v.dir.clone())
    }
}

fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parts = |s: &str| -> Vec<u64> {
        s.split(|c: char| !c.is_ascii_digit())
            .filter(|p| !p.is_empty())
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => {}
            o => return o,
        }
    }
    std::cmp::Ordering::Equal
}

pub fn installed_engines() -> Result<Vec<EngineState>> {
    let dir = paths::engines_dir();
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    for e in fs::read_dir(&dir)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().to_string();
        if name.ends_with(".json") {
            if let Ok(state) = EngineState::load(name.trim_end_matches(".json")) {
                if !state.versions.is_empty() {
                    out.push(state);
                }
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Install an engine archive (already downloaded) into the store.
/// `zip_path` may also be a bare directory when a URL extracts to one.
pub fn install_from_archive(
    def: &EngineDef,
    version: &str,
    zip_path: &Path,
    downloaded_sha256: Option<String>,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf> {
    let mut state = EngineState::load(&def.id)?;
    if state.installed(version) {
        return state.dir_for(version).ok_or_else(|| {
            HyperError::Message(format!("{} {} recorded but missing on disk", def.id, version))
        });
    }
    let target = paths::engines_dir().join(&def.id).join(sanitize_version(version));
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    fs::create_dir_all(&target)?;

    if zip_path.is_file() {
        safe_extract_zip(zip_path, &target)?;
    } else if zip_path.is_dir() {
        copy_dir(zip_path, &target, 0, &mut progress)?;
    } else {
        return Err(HyperError::NotFound(format!(
            "archive not found: {}",
            zip_path.display()
        )));
    }

    let root = find_engine_root(&target);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    state.versions.push(InstalledVersion {
        version: version.to_string(),
        dir: root.clone(),
        installed_at: now,
        downloaded_sha256,
    });
    state.save()?;
    Ok(root)
}

pub fn remove_version(def: &EngineDef, version: &str) -> Result<()> {
    let mut state = EngineState::load(&def.id)?;
    let Some(iv) = state.versions.iter().find(|v| v.version == version) else {
        return Err(HyperError::NotFound(format!(
            "{} {} is not installed",
            def.id, version
        )));
    };
    let dir = iv.dir.clone();
    state.versions.retain(|v| v.version != version);
    state.save()?;
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

/// Remove engine versions that are not pinned/used by any profile.
/// Returns the list removed. Caller must confirm before calling `do_remove`.
pub fn prune_candidates(profile_pins: &[String]) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for state in installed_engines()? {
        let used: Vec<&InstalledVersion> = state
            .versions
            .iter()
            .filter(|v| profile_pins.iter().any(|p| p == &v.version))
            .collect();
        for v in &state.versions {
            if !used.iter().any(|u| u.version == v.version) {
                out.push((state.id.clone(), v.version.clone()));
            }
        }
    }
    Ok(out)
}

fn sanitize_version(v: &str) -> String {
    let s: String = v
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() {
        "none".to_string()
    } else if s == "." || s == ".." {
        format!("v{s}")
    } else {
        s
    }
}

fn copy_dir(
    src: &Path,
    dst: &Path,
    mut count: u64,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<u64> {
    for e in fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            fs::create_dir_all(&to)?;
            count = copy_dir(&e.path(), &to, count, progress)?;
        } else {
            fs::copy(e.path(), &to)?;
            count += 1;
            if count.is_multiple_of(64) {
                progress(count, None);
            }
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_dir_safe() {
        assert_eq!(sanitize_version("1.0.4"), "1.0.4");
        assert_eq!(sanitize_version("9.0.0-pre.2"), "9.0.0-pre.2");
        assert_eq!(sanitize_version("../.."), ".._..");
        assert_eq!(sanitize_version(".."), "v..");
        assert_eq!(sanitize_version("a/b"), "a_b");
        assert_eq!(sanitize_version(""), "none");
    }
}