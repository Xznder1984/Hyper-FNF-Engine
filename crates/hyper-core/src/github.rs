use serde::Deserialize;

use crate::download::HttpClient;
use crate::error::{HyperError, Result};
use crate::keyring;

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    #[serde(rename = "browser_download_url")]
    pub url: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    #[serde(rename = "tag_name")]
    pub tag: String,
    pub name: Option<String>,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(rename = "published_at", default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

pub struct GitHub {
    pub http: HttpClient,
    token: Option<String>,
}

impl GitHub {
    pub fn new() -> Self {
        let token = keyring::get_github_token();
        GitHub {
            http: HttpClient::new(Some("HyperEngine/0.1 (cli)")),
            token,
        }
    }

    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.token = token;
        self
    }

    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }

    pub fn token(&self) -> Option<&String> {
        self.token.as_ref()
    }
}

impl Default for GitHub {
    fn default() -> Self {
        Self::new()
    }
}

/// Releases for a repo, cached with ETag/conditional requests. When a token is
/// present it is attached as an Authorization header (never logged).
impl GitHub {
    fn api_headers(&self) -> Vec<(&'static str, String)> {
        let mut h = Vec::new();
        if let Some(t) = &self.token {
            h.push(("Authorization", format!("Bearer {t}")));
        }
        h
    }

    pub fn releases(&self, repo: &str) -> Result<Vec<Release>> {
        let url = format!("https://api.github.com/repos/{repo}/releases");
        let ttl = 300u64;
        let key = format!("releases-{repo}");
        let headers = self.api_headers();
        let mut releases: Vec<Release> = self
            .http
            .get_cached_json_headers(&url, &key, ttl, &headers)?;
        releases.sort_by(|a, b| {
            b.published_at
                .cmp(&a.published_at)
                .then_with(|| b.tag.cmp(&a.tag))
        });
        Ok(releases)
    }

    pub fn latest(&self, repo: &str, include_prerelease: bool) -> Result<Option<Release>> {
        let releases = self.releases(repo)?;
        Ok(releases
            .into_iter()
            .find(|r| include_prerelease || !r.prerelease))
    }

    /// Resolve a concrete tagged release (e.g. "2.0", "v1.0.4").
    pub fn release_by_tag(&self, repo: &str, tag: &str) -> Result<Option<Release>> {
        let t = tag.trim_start_matches('v');
        let url = format!("https://api.github.com/repos/{repo}/releases/tags/{tag}");
        let key = format!("release-{repo}-{tag}");
        let headers = self.api_headers();
        match self
            .http
            .get_cached_json_headers::<Release>(&url, &key, 300, &headers)
        {
            Ok(r) => Ok(Some(r)),
            Err(HyperError::Network(msg)) if msg.contains("404") => {
                // tag probably exists without a GitHub Release; fall back to list
                Ok(self
                    .releases(repo)?
                    .into_iter()
                    .find(|r| r.tag.trim_start_matches('v') == t || r.tag == tag))
            }
            Err(e) => Err(e),
        }
    }

    /// Pick an asset for a host: first explicit pattern match, then an
    /// os-token fallback. Never guesses across os families.
    pub fn pick_asset(
        &self,
        release: &Release,
        os_key: &str,
        patterns: &[String],
    ) -> Result<Option<ReleaseAsset>> {
        // exact patterns (case-insensitive)
        for pat in patterns {
            let pl = pat.to_lowercase();
            if let Some(a) = release.assets.iter().find(|a| a.name.to_lowercase() == pl) {
                return Ok(Some(a.clone()));
            }
            if let Some(a) = release
                .assets
                .iter()
                .find(|a| a.name.to_lowercase().starts_with(&pl))
            {
                return Ok(Some(a.clone()));
            }
        }
        // os-token fallback
        let token: &[&str] = match os_key {
            "windows" => &["win", "windows"],
            "macos" => &["mac", "darwin", "osx"],
            "linux" => &["linux"],
            _ => return Ok(None),
        };
        let arch = crate::engine::arch_token().to_lowercase();
        let os = os_key.to_lowercase();
        let candidates: Vec<&ReleaseAsset> = release
            .assets
            .iter()
            .filter(|a| {
                let n = a.name.to_lowercase();
                token.iter().any(|t| n.contains(t))
            })
            .collect();
        if candidates.is_empty() {
            return Ok(None);
        }
        // prefer arch match first
        let preferred = candidates.iter().find(|a| {
            let n = a.name.to_lowercase();
            if arch == "64" {
                !n.contains("32") && n.contains(&os)
            } else {
                n.contains(&arch)
            }
        });
        Ok(Some(preferred.copied().unwrap_or(candidates[0]).clone()))
    }

    pub fn download_asset(
        &self,
        asset: &ReleaseAsset,
        dest: &std::path::Path,
        verify_sha256: Option<&str>,
        progress: impl FnMut(u64, Option<u64>),
    ) -> Result<String> {
        self.http.download(&asset.url, dest, progress)?;
        let actual = crate::download::sha256_file(dest)?;
        if let Some(expected) = verify_sha256 {
            if !expected.eq_ignore_ascii_case(&actual) {
                return Err(HyperError::Checksum {
                    expected: expected.to_string(),
                    got: actual,
                });
            }
        }
        Ok(actual)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_token_fallback_prefers_arch() {
        let gh = GitHub::new();
        let release = Release {
            tag: "t".into(),
            name: None,
            prerelease: false,
            published_at: None,
            assets: vec![
                ReleaseAsset {
                    name: "PsychEngine-Windows32.zip".into(),
                    url: "u".into(),
                    size: None,
                    digest: None,
                },
                ReleaseAsset {
                    name: "PsychEngine-Windows64.zip".into(),
                    url: "u".into(),
                    size: None,
                    digest: None,
                },
            ],
        };
        let a = gh
            .pick_asset(&release, "windows", &["PsychEngine-Windows64.zip".into()])
            .unwrap()
            .unwrap();
        assert_eq!(a.name, "PsychEngine-Windows64.zip");
    }

    #[test]
    fn no_pattern_no_token_still_finds_windows_build() {
        let gh = GitHub::new();
        let release = Release {
            tag: "t".into(),
            name: None,
            prerelease: false,
            published_at: None,
            assets: vec![ReleaseAsset {
                name: "Funkin-Windows64.zip".into(),
                url: "u".into(),
                size: None,
                digest: None,
            }],
        };
        let a = gh.pick_asset(&release, "windows", &[]).unwrap().unwrap();
        assert_eq!(a.name, "Funkin-Windows64.zip");
    }
}
