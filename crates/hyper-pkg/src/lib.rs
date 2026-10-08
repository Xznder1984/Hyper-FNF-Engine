//! Package registry support for Hyper Engine.
//!
//! A dedicated registry server is future work; the *interface* and manifest
//! format are designed now so one can be dropped in later. Today `hfe pkg`
//! supports two explicit, working sources:
//!   - `pkg add git <url> [rev]`      (clone a mod repo; requires git)
//!   - `pkg add release <url> [sha256]` (download a release archive; verified
//!     with sha256 when supplied)
//!
//! There is deliberately no fake registry: any `<name>` lookup that would
//! require one fails with a clear "registry not ready yet" error.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Git {
        url: String,
        rev: Option<String>,
    },
    Release {
        url: String,
        sha256: Option<String>,
    },
}

/// Manifest written next to a user's mod once a package is added.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub source: Source,
    pub installed_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PackageHit {
    pub name: String,
    pub engine: String,
    pub summary: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PkgError {
    #[error("package registry is not available yet; use `hfe pkg add git <url>` or `hfe pkg add release <url>` instead")]
    RegistryNotReady,
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, PkgError>;

/// Interface a future registry server will implement.
pub trait RegistryClient {
    fn name(&self) -> &'static str;
    fn search(&self, query: &str) -> Result<Vec<PackageHit>>;
    fn resolve(&self, name: &str, engine: Option<&str>) -> Result<Manifest>;
}

/// Placeholder client: fails honestly until a registry exists.
pub struct ComingSoonRegistry;

impl RegistryClient for ComingSoonRegistry {
    fn name(&self) -> &'static str {
        "coming-soon"
    }
    fn search(&self, _query: &str) -> Result<Vec<PackageHit>> {
        Err(PkgError::RegistryNotReady)
    }
    fn resolve(&self, _name: &str, _engine: Option<&str>) -> Result<Manifest> {
        Err(PkgError::RegistryNotReady)
    }
}

/// Parse `pkg add` style arguments into an explicit source.
/// Accepts: `git <url> [rev]`, `release <url> [sha256]`, `src` shorthand.
pub fn parse_add_spec(args: &[String]) -> Result<Source> {
    let kind = args.first().map(|s| s.to_ascii_lowercase());
    match kind.as_deref() {
        Some("git") | Some("src") => {
            let url = args
                .get(1)
                .ok_or_else(|| PkgError::Message("usage: hfe pkg add git <url> [rev]".into()))?
                .clone();
            let rev = args.get(2).cloned();
            Ok(Source::Git { url, rev })
        }
        Some("release") | Some("url") => {
            let url = args
                .get(1)
                .ok_or_else(|| PkgError::Message("usage: hfe pkg add release <url> [sha256]".into()))?
                .clone();
            let sha256 = args.get(2).cloned();
            Ok(Source::Release { url, sha256 })
        }
        Some("registry") | Some("name") => Err(PkgError::RegistryNotReady),
        Some(other) => Err(PkgError::Message(format!(
            "unknown source '{other}' (use: git <url> [rev] | release <url> [sha256])"
        ))),
        None => Err(PkgError::Message(
            "missing source (use: git <url> [rev] | release <url> [sha256])".into(),
        )),
    }
}

/// Serialise a Source for a human-readable line in docs/UI.
pub fn describe(source: &Source) -> String {
    match source {
        Source::Git { url, rev } => match rev {
            Some(r) => format!("git source: {url} @ {r}"),
            None => format!("git source: {url} (default branch)"),
        },
        Source::Release { url, sha256 } => match sha256 {
            Some(_) => format!("release source: {url} (sha256-verified)"),
            None => format!("release source: {url} (no published checksum; manual review advised)"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn explicit_sources_parse() {
        assert_eq!(
            parse_add_spec(&s(&["git", "https://github.com/x/y"])).unwrap(),
            Source::Git {
                url: "https://github.com/x/y".into(),
                rev: None
            }
        );
        assert_eq!(
            parse_add_spec(&s(&["release", "https://e/x.zip", "abc123"])).unwrap(),
            Source::Release {
                url: "https://e/x.zip".into(),
                sha256: Some("abc123".into())
            }
        );
    }

    #[test]
    fn registry_fails_honestly() {
        assert!(matches!(
            parse_add_spec(&s(&["name", "SomeMod"])),
            Err(PkgError::RegistryNotReady)
        ));
        assert!(ComingSoonRegistry.search("x").is_err());
    }
}