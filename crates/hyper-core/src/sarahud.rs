use std::fs;
use std::path::{Path, PathBuf};

use crate::archive::safe_extract_zip;
use crate::error::{HyperError, Result};
use crate::github::{GitHub, ReleaseAsset};
use crate::paths;

pub const CREDIT: &str = "SaraHUD by Novikond (funkin sprites by The Funkin' Crew)";
pub const CREDIT_HOMEPAGE: &str = "https://github.com/Novikond/sarahud";
pub const CREDIT_GAMEBANANA: &str = "https://gamebanana.com/mods/371851";
pub const FOLDER: &str = "SaraHUD";

/// Source release/tag per engine kind (matches engine def `sarahud` field).
pub fn source(engine_kind: &str) -> Result<(&'static str, &'static str)> {
    match engine_kind {
        "psych" => Ok(("Novikond/sarahud", "sarahud-2.0-psych.zip")),
        "fps-plus" => Ok(("Novikond/sarahud", "sarahud-2.0-fpsp.zip")),
        _ => Err(HyperError::Message(format!(
            "SaraHUD is not available for engine kind '{engine_kind}'"
        ))),
    }
}

/// Resolve the official release asset for SaraHUD.
pub fn resolve_asset(gh: &GitHub, engine_kind: &str) -> Result<ReleaseAsset> {
    let (repo, asset_pat) = source(engine_kind)?;
    let release = gh
        .release_by_tag(repo, "2.0")?
        .ok_or_else(|| HyperError::NotFound("SaraHUD release 2.0 not found".into()))?;
    let asset = gh
        .pick_asset(&release, &crate::engine::os_key(), &[asset_pat.to_string()])?
        .ok_or_else(|| {
            HyperError::NotFound(format!(
                "SaraHUD asset '{asset_pat}' not found in release {}",
                release.tag
            ))
        })?;
    Ok(asset)
}

/// Download (to cache) and extract SaraHUD into `<engine root>/mods/SaraHUD`.
/// Official source only — SaraHUD's licence requires no re-upload and credit.
pub fn install(
    gh: &GitHub,
    engine_kind: &str,
    engine_root: &Path,
    mods_dir: &str,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf> {
    let asset = resolve_asset(gh, engine_kind)?;
    let dest = paths::cache_dir().join("downloads").join(&asset.name);
    if !dest.exists() {
        gh.download_asset(&asset, &dest, None, progress)?;
    }
    let mods_root = engine_root.join(mods_dir);
    fs::create_dir_all(&mods_root)?;
    let target = mods_root.join(FOLDER);
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    // the zip usually contains a single top-level folder; replicate layout
    let scratch = paths::cache_dir().join("downloads").join("sarahud-scratch");
    let _ = fs::remove_dir_all(&scratch);
    safe_extract_zip(&dest, &scratch)?;
    let sr = crate::archive::find_engine_root(&scratch);
    if sr == scratch {
        // flat archive: files directly under scratch
        move_contents(&scratch, &target)?;
    } else {
        fs::create_dir_all(&target)?;
        move_contents(&sr, &target)?;
    }
    let _ = fs::remove_dir_all(&scratch);
    Ok(target)
}

/// Put SaraHUD first in the engine's mod order file (as the author requests).
pub fn set_first_in_order(mods_dir: &Path, order_file: Option<&str>, folder: &str) -> Result<()> {
    let Some(order_file) = order_file else {
        return Ok(());
    };
    let path = mods_dir.join(order_file);
    let mut lines: Vec<String> = Vec::new();
    if path.exists() {
        let text = fs::read_to_string(&path)?;
        for l in text.lines() {
            let t = l.trim();
            if !t.is_empty() && t != folder {
                lines.push(t.to_string());
            }
        }
    }
    lines.insert(0, folder.to_string());
    fs::write(&path, lines.join("\n") + "\n")?;
    Ok(())
}

/// Remove SaraHUD from a mods dir and its order file.
pub fn remove(mods_dir: &Path, order_file: Option<&str>, folder: &str) -> Result<()> {
    let target = mods_dir.join(folder);
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    if let Some(order_file) = order_file {
        let path = mods_dir.join(order_file);
        if path.exists() {
            let text = fs::read_to_string(&path)?;
            let lines: Vec<String> = text
                .lines()
                .filter(|l| l.trim() != folder && !l.trim().is_empty())
                .map(|l| l.to_string())
                .collect();
            fs::write(
                &path,
                lines.join("\n") + if lines.is_empty() { "" } else { "\n" },
            )?;
        }
    }
    Ok(())
}

pub fn is_installed(mods_dir: &Path, folder: &str) -> bool {
    mods_dir.join(folder).is_dir()
}

fn move_contents(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &to)?;
        } else {
            fs::copy(e.path(), &to)?;
        }
    }
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &to)?;
        } else {
            fs::copy(e.path(), &to)?;
        }
    }
    Ok(())
}

/// Draft message for the author (user sends it themselves; we never contact).
pub const AUTHOR_MESSAGE: &str = "\
Hi Novikond,

I'm building an open-source launcher (Hyper Engine) that can run FNF mods on
the correct engine version, and it supports SaraHUD for Psych Engine and FPS+.
It only ever downloads SaraHUD from your official GitHub releases, extracts it
into the engine's mods folder, and sets it first in the mod order as your
README asks. The launcher credits and links your GitHub and GameBanana pages.

If you'd prefer a different approach, or you'd like me to remove the
integration, just reply and I'll adjust.

Thanks for making SaraHUD available!";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_file_prepend_and_remove() {
        let dir = std::env::temp_dir().join(format!("hfe-sarahud-{}", std::process::id()));
        let mods = dir.join("mods");
        fs::create_dir_all(&mods).unwrap();
        fs::write(mods.join("modsList.txt"), "buy-the-dip\ncool-mod\n").unwrap();
        set_first_in_order(&mods, Some("modsList.txt"), FOLDER).unwrap();
        let text = fs::read_to_string(mods.join("modsList.txt")).unwrap();
        assert!(text.lines().next() == Some(FOLDER));
        remove(&mods, Some("modsList.txt"), FOLDER).unwrap();
        let text2 = fs::read_to_string(mods.join("modsList.txt")).unwrap();
        assert!(text2.lines().all(|l| l != FOLDER));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_two_engines_supported() {
        assert!(source("psych").is_ok());
        assert!(source("fps-plus").is_ok());
        assert!(source("codename").is_err());
    }
}
