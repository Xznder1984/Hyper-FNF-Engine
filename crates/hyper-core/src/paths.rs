use std::path::PathBuf;

/// Data directory: installed engines, settings, profiles.
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hyper-engine")
}

/// Config directory: engine definition overrides, tokens.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hyper-engine")
}

/// Cache directory: downloads, release metadata, GameBanana lookups.
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| data_dir().join("cache"))
        .join("hyper-engine")
}

pub fn engines_dir() -> PathBuf {
    data_dir().join("engines")
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn profiles_dir() -> PathBuf {
    data_dir().join("profiles")
}

pub fn rollback_dir() -> PathBuf {
    data_dir().join("update").join("previous")
}

pub fn launcher_manifest_path() -> PathBuf {
    data_dir().join("update").join("installed.json")
}

pub fn default_downloads_dir() -> PathBuf {
    cache_dir().join("downloads")
}

pub fn github_cache_dir() -> PathBuf {
    cache_dir().join("github")
}

pub fn gamebanana_cache_dir() -> PathBuf {
    cache_dir().join("gamebanana")
}

/// Override directory for community engine definitions (config_dir/engines).
pub fn external_engines_dir() -> PathBuf {
    config_dir().join("engines")
}

/// env override used by tests
pub fn engines_env_override() -> Option<PathBuf> {
    std::env::var_os("HYPER_ENGINES_DIR").map(PathBuf::from)
}
