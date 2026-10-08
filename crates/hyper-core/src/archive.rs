use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::{HyperError, Result};

/// Extract `zip_path` into `dest`, refusing any entry that escapes `dest`
/// (zip-slip guard) and skipping directory entries.
pub fn safe_extract_zip(zip_path: &Path, dest: &Path) -> Result<Vec<String>> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    fs::create_dir_all(dest)?;
    let mut extracted = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let raw_name = entry.name().to_string();
        let name = raw_name.replace('\\', "/");
        let rel = Path::new(&name);
        if !rel.is_relative() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(HyperError::Zip(format!(
                "unsafe path in archive: {raw_name}"
            )));
        }
        if entry.is_dir() {
            continue;
        }
        let target = dest.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&target)?;
        std::io::copy(&mut entry, &mut out)?;
        extracted.push(name);
    }
    Ok(extracted)
}

/// Locate a mod folder inside an extracted engine tree or return the mods dir.
/// Some engines extract into a single top-level folder; helper to find the
/// actual engine root.
pub fn find_engine_root(root: &Path) -> PathBuf {
    if let Ok(read) = fs::read_dir(root) {
        let dirs: Vec<PathBuf> = read
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect();
        if dirs.len() == 1 {
            let only = &dirs[0];
            let name = only
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            // a single top-level directory that itself looks like an engine
            if name != "mods" && !name.starts_with('.') {
                return only.clone();
            }
        }
    }
    root.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn extracts_clean_zip() {
        let dir = std::env::temp_dir().join(format!("hfe-zip-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let inner = dir.join("inner");
        fs::create_dir_all(inner.join("data")).unwrap();
        fs::create_dir_all(inner.join("mods").join("m")).unwrap();
        fs::write(inner.join("data").join("x.lua"), "x").unwrap();
        fs::write(inner.join("mods").join("m").join("pack.json"), "{}").unwrap();

        let real = dir.join("real.zip");
        build_test_zip(&real, &inner).unwrap();
        let out = dir.join("out");
        let extracted = safe_extract_zip(&real, &out).unwrap();
        assert!(extracted.iter().any(|f| f.ends_with("x.lua")));
        assert!(out.join("mods").join("m").join("pack.json").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    fn build_test_zip(zip_path: &Path, src: &Path) -> Result<()> {
        use std::io::Read;
        let file = fs::File::create(zip_path)?;
        let mut w = zip::ZipWriter::new(file);
        let opts = zip::write::FileOptions::default();
        let mut add = |p: &Path| -> Result<()> {
            let rel = p
                .strip_prefix(src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if p.is_dir() {
                w.add_directory(format!("{rel}/"), opts)?;
            } else {
                w.start_file(rel, opts)?;
                let mut f = fs::File::open(p)?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)?;
                w.write_all(&buf)?;
            }
            Ok(())
        };
        let mut stack = vec![src.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for e in fs::read_dir(&dir)? {
                let e = e?;
                if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    stack.push(e.path());
                }
                add(&e.path())?;
            }
        }
        w.finish()?;
        Ok(())
    }

    #[test]
    fn rejects_parent_escape() {
        let dir = std::env::temp_dir().join(format!("hfe-zip-bad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("bad.zip");
        let f = fs::File::create(&zip_path).unwrap();
        let mut w = zip::ZipWriter::new(f);
        w.start_file("../evil.txt", zip::write::FileOptions::default())
            .unwrap();
        let _ = w.write_all(b"evil");
        let _ = w.finish();
        let res = safe_extract_zip(&zip_path, &dir.join("out"));
        assert!(res.is_err(), "zip-slip must be rejected");
        let _ = fs::remove_dir_all(&dir);
    }
}
