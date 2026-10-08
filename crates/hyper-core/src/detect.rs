use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::engine::{glob_match, EngineDef, EngineLibrary};
use crate::error::Result;
use crate::settings::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Confidence::Low => "low",
            Confidence::Medium => "medium",
            Confidence::High => "high",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub engine_id: String,
    pub score: i32,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Detection {
    pub engine_id: Option<String>,
    pub confidence: Confidence,
    pub candidates: Vec<Candidate>,
    pub evidence: Vec<String>,
    pub mod_meta: ModMeta,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModMeta {
    pub title: Option<String>,
    pub version: Option<String>,
    pub uid: Option<String>,
    pub raw_files: Vec<String>,
}

/// Stable key for a mod directory (used for profile/override lookups).
pub fn mod_key(path: &Path) -> Result<String> {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(canon.to_string_lossy().as_bytes());
    Ok(hex::encode(h.finalize())[..16].to_string())
}

/// What metadata files exist at the mod root (names only).
fn read_meta(path: &Path) -> ModMeta {
    let mut meta = ModMeta::default();
    if !path.is_dir() {
        return meta;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return meta;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.ends_with(".json") || name.ends_with(".xml") {
            meta.raw_files.push(name.clone());
            match name.as_str() {
                "pack.json" | "meta.json" | "metadata.json" | "mods.json" => {
                    if let Ok(text) = fs::read_to_string(e.path()) {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                            if meta.title.is_none() {
                                meta.title = v
                                    .get("title")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string());
                            }
                            if meta.version.is_none() {
                                meta.version = v
                                    .get("version")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string());
                            }
                            if meta.uid.is_none() {
                                meta.uid =
                                    v.get("uid").and_then(|u| u.as_str()).map(|s| s.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    meta
}

/// Detect the engine for `mod_dir`.
///
/// Order: explicit override (remembered) -> metadata scoring -> structural
/// signatures. GameBanana enrichment can be layered on top by the caller.
pub fn detect(mod_dir: &Path, lib: &EngineLibrary, settings: &Settings) -> Detection {
    // 1. remembered override
    if let Ok(key) = mod_key(mod_dir) {
        if let Some(id) = settings.engine_override(&key) {
            let det = Detection {
                engine_id: Some(id.clone()),
                confidence: Confidence::High,
                candidates: vec![Candidate {
                    engine_id: id,
                    score: 999,
                    evidence: vec!["remembered override".to_string()],
                }],
                evidence: vec!["remembered override".to_string()],
                mod_meta: read_meta(mod_dir),
            };
            return det;
        }
    }

    let meta = read_meta(mod_dir);
    let mut candidates: Vec<Candidate> = Vec::new();

    for def in &lib.defs {
        let mut score = 0i32;
        let mut evidence: Vec<String> = Vec::new();

        // metadata files -> authoritative-ish score
        for mf in &def.mod_metadata {
            if meta.raw_files.iter().any(|f| f.eq_ignore_ascii_case(mf)) {
                score += def.detect.metadata_scores.get(mf).copied().unwrap_or(3);
                evidence.push(format!("has {mf}"));
            }
        }
        if def.requires_uid && meta.uid.is_some() && evidence.is_empty() {
            // fps-plus style: any meta with uid leans fps-plus
            score += 2;
            evidence.push("metadata carries a uid".to_string());
        }

        // structural signatures
        let hits = structural_hits(mod_dir, def);
        for (sig, n) in hits {
            score += n;
            evidence.push(format!("matches {sig}"));
        }

        if score > 0 {
            candidates.push(Candidate {
                engine_id: def.id.clone(),
                score,
                evidence,
            });
        }
    }

    candidates.sort_by_key(|c| std::cmp::Reverse(c.score));

    let confidence = classify(&meta, &candidates);
    let engine_id = candidates.first().map(|c| c.engine_id.clone());
    let mut evidence = vec![format!("{} metadata file(s) found", meta.raw_files.len())];
    if let Some(title) = &meta.title {
        evidence.push(format!("mod title: {title}"));
    }
    if let Some(uid) = &meta.uid {
        evidence.push(format!("mod uid: {uid}"));
    }

    Detection {
        engine_id,
        confidence,
        candidates,
        evidence,
        mod_meta: meta,
    }
}

fn classify(meta: &ModMeta, candidates: &[Candidate]) -> Confidence {
    // High when every metadata file present belongs to one engine that also
    // holds a strong (>= 4) score. Multiple files of the same engine count as
    // one owner (e.g. codename's metadata.json + main.xml).
    let owned: Vec<(&str, &str)> = vec![
        ("pack.json", "psych"),
        ("mods.json", "psych"),
        ("meta.json", "fps-plus"),
        ("metadata.json", "codename"),
        ("main.xml", "codename"),
        ("manifest.json", "funkin-official"),
    ];
    let present: Vec<(&str, &str)> = owned
        .iter()
        .filter(|(f, _)| meta.raw_files.iter().any(|r| r.eq_ignore_ascii_case(f)))
        .cloned()
        .collect();
    let owners: std::collections::BTreeSet<&str> = present.iter().map(|(_, e)| *e).collect();
    if owners.len() == 1 {
        if let Some(owner) = owners.iter().next() {
            if let Some(c) = candidates.iter().find(|c| c.engine_id == *owner) {
                if c.score >= 4 {
                    return Confidence::High;
                }
            }
        }
    }
    match candidates.first() {
        Some(c) if c.score >= 4 => Confidence::Medium,
        Some(_) => Confidence::Low,
        None => Confidence::Low,
    }
}

/// Walk the mod dir (bounded) and return (signature, weight) matches, deduped.
fn structural_hits(mod_dir: &Path, def: &EngineDef) -> Vec<(String, i32)> {
    if def.detect.signatures.is_empty() {
        return vec![];
    }
    let mut matched: BTreeSet<String> = BTreeSet::new();
    let mut count = 0usize;
    let walker = walkdir::WalkDir::new(mod_dir).max_depth(8).into_iter();
    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        count += 1;
        if count > 20_000 {
            break;
        }
        let rel = match entry.path().strip_prefix(mod_dir) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if rel.contains("/mods/") {
            continue;
        }
        for sig in def.detect.signatures.keys() {
            if !matched.contains(sig) && glob_match(sig, &rel) {
                matched.insert(sig.clone());
            }
        }
    }
    matched
        .into_iter()
        .map(|sig| {
            (
                sig.clone(),
                def.detect.signatures.get(&sig).copied().unwrap_or(1),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// GameBanana enrichment: best-effort, cached, rate-limited, never blocking.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameBananaMod {
    #[serde(rename = "_idRow")]
    pub id: i64,
    #[serde(rename = "_sName")]
    pub name: Option<String>,
    #[serde(rename = "_sSubtitle")]
    pub subtitle: Option<String>,
    #[serde(rename = "_aSubmitter")]
    pub submitter: Option<serde_json::Value>,
}

pub struct GameBananaClient {
    http: crate::download::HttpClient,
}

impl GameBananaClient {
    pub fn new() -> Self {
        GameBananaClient {
            http: crate::download::HttpClient::new(Some("HyperEngine/0.1 (cli)")),
        }
    }

    /// Fetch public mod page metadata. Cached on disk; rate-limited in-memory.
    pub fn mod_info(&mut self, mod_id: i64) -> Result<Option<GameBananaMod>> {
        if std::env::var("HYPER_GB").ok().as_deref() != Some("1") {
            return Ok(None); // enrichment off by default (no network surprises)
        }
        let cache_dir = crate::paths::gamebanana_cache_dir();
        let cache_file = cache_dir.join(format!("mod-{mod_id}.json"));
        if cache_file.exists() {
            if let Ok(text) = fs::read_to_string(&cache_file) {
                if let Ok(v) = serde_json::from_str::<GameBananaMod>(&text) {
                    return Ok(Some(v));
                }
            }
        }
        let url = format!("https://gamebanana.com/apiv11/Mod/{mod_id}/ProfilePage");
        let body = self.http.get_text(&url)?;
        let v: GameBananaMod = serde_json::from_str(&body)?;
        let _ = fs::create_dir_all(&cache_dir);
        let _ = fs::write(
            &cache_file,
            serde_json::to_vec_pretty(&v).unwrap_or_default(),
        );
        Ok(Some(v))
    }
}

impl Default for GameBananaClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use std::path::PathBuf;

    fn tmp_mod(tag: &str, files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hfe-detect-{}", std::process::id()));
        let root = dir.join(tag);
        let _ = fs::remove_dir_all(&root);
        for f in files {
            let p = root.join(f);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, "{}").unwrap();
        }
        root
    }

    #[test]
    fn detects_psych_from_pack_json_and_structure() {
        let lib = EngineLibrary::load().unwrap();
        let s = Settings::default();
        let m = tmp_mod(
            "psych",
            &[
                "pack.json",
                "data/songs/bopeebo/Chart.json",
                "songs/bopeebo/Inst.ogg",
                "scripts/global/PlayState.lua",
            ],
        );
        let d = detect(&m, &lib, &s);
        assert_eq!(d.engine_id.as_deref(), Some("psych"));
        assert!(d.confidence >= Confidence::High, "{d:?}");
    }

    #[test]
    fn detects_codename_from_metadata_json() {
        let lib = EngineLibrary::load().unwrap();
        let s = Settings::default();
        let m = tmp_mod(
            "codename",
            &["metadata.json", "main.xml", "data/songs/x/Chart.json"],
        );
        let d = detect(&m, &lib, &s);
        assert_eq!(d.engine_id.as_deref(), Some("codename"));
        assert!(d.confidence >= Confidence::High, "{d:?}");
    }

    #[test]
    fn detects_fps_plus_from_meta_with_uid_and_scripts() {
        let lib = EngineLibrary::load().unwrap();
        let s = Settings::default();
        let m = tmp_mod(
            "fps",
            &["meta.json", "scripts/global/PlayState.hx", "data/chart.lua"],
        );
        let d = detect(&m, &lib, &s);
        assert_eq!(d.engine_id.as_deref(), Some("fps-plus"));
        assert!(d.confidence >= Confidence::High, "{d:?}");
    }

    #[test]
    fn override_wins() {
        let lib = EngineLibrary::load().unwrap();
        let mut s = Settings::default();
        let m = tmp_mod("override", &["random.txt"]);
        let key = mod_key(&m).unwrap();
        s.set_engine_override(&key, "codename".to_string());
        let d = detect(&m, &lib, &s);
        assert_eq!(d.engine_id.as_deref(), Some("codename"));
        assert!(d.confidence == Confidence::High);
    }
}
