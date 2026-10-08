use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::download::HttpClient;
use crate::error::{HyperError, Result};
use crate::launch::{current_exe, managed_install};
use crate::paths;
use crate::settings::Settings;

pub const DEFAULT_MANIFEST_URL: &str =
    "https://github.com/Xznder1984/Hyper-FNF-Engine/releases/latest/download/update.json";

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    #[serde(default)]
    pub notes: Option<String>,
    /// major releases are never installed silently
    #[serde(default)]
    pub major: bool,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    #[serde(default = "default_platform")]
    pub platform: String,
    #[serde(default = "default_arch")]
    pub arch: String,
    pub url: String,
    #[serde(default)]
    pub sha256: Option<String>,
}

fn default_platform() -> String {
    crate::engine::os_key()
}
fn default_arch() -> String {
    crate::engine::arch_token().to_string()
}

#[derive(Debug, Clone)]
pub struct AvailableUpdate {
    pub manifest: UpdateManifest,
    pub artifact: Option<Artifact>,
    pub reason: String,
}

/// Check for an update. Returns the update when one exists and the user's
/// settings permit installing it (opt-out, no silent majors, no self-update
/// for package-manager installs).
pub fn check(
    settings: &Settings,
    http: &HttpClient,
    manifest_url: &str,
) -> Result<Option<AvailableUpdate>> {
    if !settings.update.enabled {
        return Ok(None);
    }
    if let Some(mgr) = managed_install() {
        return Err(HyperError::Message(format!(
            "hyper was installed by {mgr}; update it through the package manager \
             (in-app self-update is disabled for managed installs)"
        )));
    }
    let manifest: UpdateManifest = http.get_cached_json(manifest_url, "update-manifest", 300)?;
    let installed = read_installed_version();
    let ours = installed.as_deref().unwrap_or(env!("CARGO_PKG_VERSION"));
    if manifest.version == ours {
        return Ok(None);
    }
    if manifest.major && !settings.update.allow_major {
        return Ok(Some(AvailableUpdate {
            manifest: manifest.clone(),
            artifact: None,
            reason: format!(
                "newer major version {} available (enable major updates in settings to install)",
                manifest.version
            ),
        }));
    }
    let artifact = manifest
        .artifacts
        .iter()
        .find(|a| {
            a.platform == crate::engine::os_key()
                && (a.arch == crate::engine::arch_token() || a.arch == "*")
        })
        .cloned();
    let reason = format!("update {} -> {}", ours, manifest.version);
    Ok(Some(AvailableUpdate {
        manifest,
        artifact,
        reason,
    }))
}

/// Verify + stage an update artifact. Returns the path of the verified new
/// binary. Does NOT touch the running executable yet (see `install`).
pub fn fetch_and_verify(
    artifact: &Artifact,
    http: &HttpClient,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf> {
    let dest = paths::cache_dir().join("update").join("candidate.bin");
    if dest.exists() {
        let _ = fs::remove_file(&dest);
    }
    http.download(&artifact.url, &dest, progress)?;
    if let Some(expected) = &artifact.sha256 {
        let actual = crate::download::sha256_file(&dest)?;
        if !expected.eq_ignore_ascii_case(&actual) {
            return Err(HyperError::Checksum {
                expected: expected.clone(),
                got: actual,
            });
        }
    }
    Ok(dest)
}

/// Replace the on-disk launcher binary with the verified candidate.
/// A Windows running image cannot be overwritten, but it *can* be renamed, so
/// `replace_file` moves the live file aside first and puts the new one in its
/// place; the old copy is dropped once it is no longer mapped.
pub fn install_verified(candidate: &Path) -> Result<()> {
    let exe = current_exe();
    // back up current binary for rollback
    let prev = paths::rollback_dir().join(format!(
        "hyper-{}.bin",
        read_installed_version().unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
    ));
    if let Some(parent) = prev.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&exe, &prev)?;
    replace_file(candidate, &exe)?;
    let _ = fs::remove_file(candidate);
    Ok(())
}

/// Move `src` over `dest`, tolerating a live/locked `dest`.
fn replace_file(src: &Path, dest: &Path) -> Result<()> {
    // fast path: same volume and dest is not locked
    if fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    let aside = aside_path(dest);
    let _ = fs::remove_file(&aside);
    if fs::rename(dest, &aside).is_err() {
        // dest could not be moved aside; direct write is the last resort
        return fs::copy(src, dest).map(|_| ()).map_err(|e| {
            HyperError::Message(format!(
                "could not replace {}: {e}. Close the launcher and retry, or rollback \
                 with 'hfe update rollback'",
                dest.display()
            ))
        });
    }
    let placed = fs::rename(src, dest).or_else(|_| fs::copy(src, dest).map(|_| ()));
    match placed {
        Ok(()) => {
            // the old image may still be mapped (we are that process); harmless
            let _ = fs::remove_file(&aside);
            Ok(())
        }
        Err(e) => {
            let _ = fs::rename(&aside, dest);
            Err(HyperError::Message(format!(
                "could not replace {}: {e}. The previous binary was restored; close the \
                 launcher and retry",
                dest.display()
            )))
        }
    }
}

/// Where the live binary goes while its replacement moves in (`hfe.exe.old`).
fn aside_path(dest: &Path) -> PathBuf {
    let mut aside = dest.as_os_str().to_owned();
    aside.push(".old");
    PathBuf::from(aside)
}

/// Restore the most recently written rollback backup over the current binary
/// and put the version record back in sync with what we just restored.
pub fn rollback() -> Result<PathBuf> {
    let backup = newest_backup_in(&paths::rollback_dir())?;
    let exe = current_exe();
    replace_file(&backup, &exe)?;
    if let Some(v) = version_from_backup(&backup) {
        let _ = record_installed(&v);
    }
    Ok(backup)
}

/// Pick the most recently written `hyper-*.bin` backup inside `dir`.
fn newest_backup_in(dir: &Path) -> Result<PathBuf> {
    if !dir.is_dir() {
        return Err(HyperError::NotFound("no rollback backup found".into()));
    }
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.extension().map(|ext| ext == "bin").unwrap_or(false) {
            let mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            if newest.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
                newest = Some((mtime, path));
            }
        }
    }
    match newest {
        Some((_, path)) => Ok(path),
        None => Err(HyperError::NotFound("no rollback backup found".into())),
    }
}

/// Recover the version a backup was written for (`hyper-1.2.3.bin`).
fn version_from_backup(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let version = name.strip_prefix("hyper-")?.strip_suffix(".bin")?;
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

pub fn record_installed(version: &str) -> Result<()> {
    let p = paths::launcher_manifest_path();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&p, version)?;
    Ok(())
}

pub fn read_installed_version() -> Option<String> {
    fs::read_to_string(paths::launcher_manifest_path())
        .ok()
        .map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_artifacts_match_host() {
        let manifest = UpdateManifest {
            version: "9.9.9".into(),
            notes: None,
            major: false,
            artifacts: vec![Artifact {
                platform: crate::engine::os_key(),
                arch: crate::engine::arch_token().into(),
                url: "https://127.0.0.1/x".into(),
                sha256: None,
            }],
        };
        let hit = manifest
            .artifacts
            .iter()
            .find(|a| {
                a.platform == crate::engine::os_key()
                    && (a.arch == crate::engine::arch_token() || a.arch == "*")
            })
            .is_some();
        assert!(hit);
    }

    #[test]
    fn backup_round_trip_picks_latest_and_restores_version() {
        let dir = std::env::temp_dir().join(format!("hfe-update-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let older = dir.join("hyper-0.1.0.bin");
        let newer = dir.join("hyper-9.9.9.bin");
        fs::write(&older, b"old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        fs::write(&newer, b"new").unwrap();
        fs::write(dir.join("notes.txt"), b"not a backup").unwrap();

        assert_eq!(newest_backup_in(&dir).unwrap(), newer);
        assert_eq!(version_from_backup(&older).as_deref(), Some("0.1.0"));
        assert_eq!(version_from_backup(&newer).as_deref(), Some("9.9.9"));
        assert_eq!(version_from_backup(&dir.join("hyper-.bin")), None);
        assert_eq!(version_from_backup(&dir.join("notes.txt")), None);

        let missing = dir.join("does-not-exist");
        assert!(matches!(
            newest_backup_in(&missing),
            Err(HyperError::NotFound(_))
        ));
        let _ = fs::remove_dir_all(&dir);
    }
}
