use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::{HyperError, Result};
use crate::paths;

include!(concat!(env!("OUT_DIR"), "/engines_generated.rs"));

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EngineDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub mods_dir: String,
    #[serde(default)]
    pub order_file: Option<String>,
    /// Metadata files that live inside each mod folder, most specific first.
    #[serde(default)]
    pub mod_metadata: Vec<String>,
    #[serde(default)]
    pub requires_uid: bool,
    #[serde(default)]
    pub script_warning: bool,
    /// "psych" or "fps-plus" when this engine supports SaraHUD.
    #[serde(default)]
    pub sarahud: Option<String>,
    #[serde(default)]
    pub needs_base_assets: bool,
    /// Windows or macOS build note shown to the user.
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub asset_patterns: BTreeMap<String, AssetPatterns>,
    /// Detection tuning, nested under `[detect]` in the TOML definitions.
    #[serde(default)]
    pub detect: DetectTuning,
    /// Engine-exposed CLI/environment options the launcher may set.
    #[serde(default)]
    pub exposed_options: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DetectTuning {
    /// Glob -> weight matches walked inside a mod dir.
    #[serde(default)]
    pub signatures: BTreeMap<String, i32>,
    /// Metadata file -> score for files listed in `mod_metadata`.
    #[serde(default)]
    pub metadata_scores: BTreeMap<String, i32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AssetPatterns {
    #[serde(default)]
    pub patterns: Vec<String>,
}

impl EngineDef {
    pub fn from_toml(text: &str) -> Result<Self> {
        let def: Self = toml::from_str(text)
            .map_err(|e| HyperError::Message(format!("bad engine definition: {e}")))?;
        if def.id.is_empty() {
            return Err(HyperError::Message("engine definition missing id".into()));
        }
        Ok(def)
    }

    /// Asset name for a Host (os key like "windows"). Returns the first pattern
    /// whose (lowercased) name contains the os token. Empty patterns mean
    /// "any asset matching the os token" handled by the caller.
    pub fn asset_candidates(&self, os_key: &str) -> Vec<String> {
        let Some(ap) = self.asset_patterns.get(os_key) else {
            return vec![];
        };
        ap.patterns.clone()
    }
}

/// A loaded engine library: embedded defaults + external overrides.
#[derive(Debug, Clone, Default)]
pub struct EngineLibrary {
    pub defs: Vec<EngineDef>,
}

impl EngineLibrary {
    pub fn load() -> Result<Self> {
        let mut lib = EngineLibrary::default();
        let host_os = os_key();

        // Embedded definitions, ordered by id. A later definition with the
        // same id replaces an earlier one (external overrides embedded).
        let mut by_id: BTreeMap<String, EngineDef> = BTreeMap::new();
        for (id, text) in embedded() {
            let def = EngineDef::from_toml(text)?;
            if def.id == id {
                by_id.insert(id, def);
            }
        }
        load_dir_into(&mut by_id, &paths::external_engines_dir())?;
        if let Some(dir) = paths::engines_env_override() {
            load_dir_into(&mut by_id, &dir)?;
        }
        lib.defs = by_id.into_values().collect();
        lib.defs.retain(|d| d.asset_patterns.contains_key(&host_os) || d.repo.is_some());
        Ok(lib)
    }

    pub fn get(&self, id: &str) -> Option<&EngineDef> {
        self.defs.iter().find(|d| d.id == id)
    }

    pub fn ids(&self) -> Vec<String> {
        self.defs.iter().map(|d| d.id.clone()).collect()
    }
}

fn embedded() -> Vec<(String, &'static str)> {
    let v = vec![
        (String::from("psych"), PSYCH),
        (String::from("codename"), CODENAME),
        (String::from("fps-plus"), FPS_PLUS),
        (String::from("funkin-official"), FUNKIN_OFFICIAL),
    ];
    v
}

fn load_dir_into(map: &mut BTreeMap<String, EngineDef>, dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let p = entry?.path();
        if p.extension().map(|x| x == "toml").unwrap_or(false) {
            let text = fs::read_to_string(&p)?;
            let def = EngineDef::from_toml(&text)?;
            map.insert(def.id.clone(), def);
        }
    }
    Ok(())
}

/// Normalise "windows"/"macos"/"linux" for asset matching.
pub fn os_key() -> String {
    #[cfg(target_os = "windows")]
    {
        return "windows".to_string();
    }
    #[cfg(target_os = "macos")]
    {
        return "macos".to_string();
    }
    #[cfg(target_os = "linux")]
    {
        return "linux".to_string();
    }
    #[allow(unreachable_code)]
    {
        std::env::consts::OS.to_string()
    }
}

pub fn arch_token() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "64",
        "aarch64" => "arm64",
        "x86" => "32",
        other => other,
    }
}

/// Convert a glob (with `**` and `*`) to a regular expression string.
pub fn glob_to_regex(glob: &str) -> String {
    let mut re = String::from("^");
    let chars: Vec<char> = glob.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    re.push_str(".*");
                    i += 1;
                } else {
                    re.push_str("[^/]*");
                }
            }
            '?' => re.push_str("[^/]"),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                re.push('\\');
                re.push(chars[i]);
            }
            c => re.push(c),
        }
        i += 1;
    }
    re.push('$');
    re
}

/// Test if a relative path (always `/`-separated) matches a glob.
pub fn glob_match(glob: &str, rel_path: &str) -> bool {
    let re = glob_to_regex(glob);
    match regex::Regex::new(&re) {
        Ok(r) => r.is_match(rel_path),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_embedded_psych() {
        let lib = EngineLibrary::load().unwrap();
        let ids = lib.ids();
        assert!(ids.iter().any(|i| i == "psych"), "psych def missing: {ids:?}");
    }

    #[test]
    fn glob_matching() {
        assert!(glob_match("data/**", "data/songs/bopeebo/Chart.json"));
        assert!(glob_match("data/**", "data/chart.lua"));
        assert!(!glob_match("data/**", "songs/bopeebo/Chart.json"));
        assert!(glob_match("scripts/**/*.lua", "scripts/global/PlayState.lua"));
        assert!(glob_match("**/*.json", "a/b/c.json"));
    }
}