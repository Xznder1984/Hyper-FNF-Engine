use std::collections::BTreeMap;
use std::fs;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// mod_key -> engine id (remembered manual choice)
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
    /// mod_key -> pinned engine version
    #[serde(default)]
    pub pinned: BTreeMap<String, String>,
    /// mod_key -> launcher profile
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
    #[serde(default)]
    pub update: UpdateSettings,
    #[serde(default)]
    pub sarahud: SarahudSettings,
    /// Optional override for where engines are installed.
    #[serde(default)]
    pub data_dir: Option<String>,
    /// Path to a base game install (e.g. the official FNF Windows build) used
    /// for engines that need external assets.
    #[serde(default)]
    pub game_assets: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub auto_check: bool,
    #[serde(default)]
    pub allow_major: bool,
}

fn default_true() -> bool {
    true
}

/// Fresh installs have no settings file yet, so they must match the serde
/// default: update checks are on until the user runs `hfe update disable`.
impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_check: false,
            allow_major: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SarahudSettings {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    #[serde(default)]
    pub engine_version: Option<String>,
    /// "low", "normal", "high", "realtime" (launcher-level process priority)
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl Settings {
    pub fn load() -> Result<Self> {
        let p = paths::settings_path();
        if !p.exists() {
            return Ok(Settings::default());
        }
        let text = fs::read_to_string(&p)?;
        match serde_json::from_str(&text) {
            Ok(s) => Ok(s),
            Err(e) => Err(crate::error::HyperError::Message(format!(
                "settings at {} are unreadable ({e}); fix or delete the file",
                p.display()
            ))),
        }
    }

    pub fn save(&self) -> Result<()> {
        let p = paths::settings_path();
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self)?;
        fs::write(&p, json)?;
        Ok(())
    }

    pub fn engine_override(&self, mod_key: &str) -> Option<String> {
        self.overrides.get(mod_key).cloned()
    }

    pub fn set_engine_override(&mut self, mod_key: &str, engine_id: String) {
        self.overrides.insert(mod_key.to_string(), engine_id);
    }

    pub fn remove_override(&mut self, mod_key: &str) {
        self.overrides.remove(mod_key);
    }

    pub fn profile_for(&self, mod_key: &str) -> Profile {
        self.profiles.get(mod_key).cloned().unwrap_or_default()
    }

    pub fn set_profile(&mut self, mod_key: &str, p: Profile) {
        self.profiles.insert(mod_key.to_string(), p);
    }

    pub fn pinned_version(&self, mod_key: &str) -> Option<String> {
        self.pinned.get(mod_key).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut s = Settings::default();
        s.set_engine_override("abc", "psych".into());
        s.save().unwrap();
        let s2 = Settings::load().unwrap();
        assert_eq!(s2.engine_override("abc").as_deref(), Some("psych"));
        let _ = fs::remove_file(paths::settings_path());
    }

    #[test]
    fn fresh_install_defaults_match_serde_defaults() {
        assert!(
            Settings::default().update.enabled,
            "checks on for fresh installs"
        );
        let empty: Settings = serde_json::from_str("{}").unwrap();
        assert!(
            empty.update.enabled,
            "serde default must agree with Default"
        );
        assert!(!empty.update.auto_check);
        assert!(!empty.update.allow_major);
    }
}
