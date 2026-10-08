use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{HyperError, Result};
use crate::paths;

/// HTTP(S) client enforcing secure-by-default policy. Plain http is allowed
/// only for loopback hosts (tests, local fake-update servers).
pub struct HttpClient {
    client: reqwest::blocking::Client,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedResource {
    pub etag: Option<String>,
    pub fetched_at: u64,
    pub body: Vec<u8>,
}

impl HttpClient {
    pub fn new(user_agent: Option<&str>) -> Self {
        let client = reqwest::blocking::Client::builder()
            .user_agent(user_agent.unwrap_or("HyperEngine/0.1"))
            .timeout(Duration::from_secs(900))
            .connect_timeout(Duration::from_secs(30))
            .gzip(true)
            .brotli(true)
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        HttpClient { client }
    }

    pub fn assert_allowed(url: &str) -> Result<()> {
        let u = url::Url::parse(url)
            .map_err(|e| HyperError::Network(format!("bad url {url}: {e}")))?;
        match u.scheme() {
            "https" => Ok(()),
            "http" => {
                let host = u.host_str().unwrap_or("");
                if host == "127.0.0.1" || host == "localhost" || host == "[::1]" {
                    Ok(())
                } else {
                    Err(HyperError::Network(
                        "refusing plain-http download (https required)".into(),
                    ))
                }
            }
            s => Err(HyperError::Network(format!("unsupported scheme {s}"))),
        }
    }

    pub fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        Self::assert_allowed(url)?;
        let mut resp = self
            .client
            .get(url)
            .send()
            .map_err(|e| HyperError::Network(format!("GET {url}: {e}")))?;
        if !resp.status().is_success() {
            return Err(HyperError::Network(format!(
                "GET {url}: HTTP {}",
                resp.status()
            )));
        }
        let mut buf = Vec::new();
        resp.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn get_text(&self, url: &str) -> Result<String> {
        let bytes = self.get_bytes(url)?;
        String::from_utf8(bytes)
            .map_err(|_| HyperError::Network("response is not utf-8 text".into()))
    }

    /// Max-depth node for ETag-cached JSON resources (GitHub API, manifests).
    pub fn get_cached_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        key: &str,
        ttl_secs: u64,
    ) -> Result<T> {
        self.get_cached_json_headers::<T>(url, key, ttl_secs, &[])
    }

    /// Like `get_cached_json` but with extra headers per request (e.g. auth).
    pub fn get_cached_json_headers<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        key: &str,
        ttl_secs: u64,
        extra: &[(&str, String)],
    ) -> Result<T> {
        Self::assert_allowed(url)?;
        let cache_file = cache_file(key);
        let cached = read_cache(&cache_file);

        let hit_ok = cached
            .as_ref()
            .map(|c| {
                let age = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                age.saturating_sub(c.fetched_at) < ttl_secs && !c.body.is_empty()
            })
            .unwrap_or(false);

        let mut req = self.client.get(url);
        if let Some(c) = &cached {
            if let Some(etag) = &c.etag {
                req = req.header("If-None-Match", etag.clone());
            }
        }
        for (k, v) in extra {
            req = req.header(*k, v.as_str());
        }
        let mut resp = req
            .send()
            .map_err(|e| HyperError::Network(format!("GET {url}: {e}")))?;

        if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
            if let Some(cached) = &cached {
                return serde_json::from_slice(&cached.body).map_err(HyperError::from);
            }
        }
        if !resp.status().is_success() {
            if hit_ok {
                // stale cache is better than nothing; keep it
                return serde_json::from_slice(&cached.as_ref().unwrap().body)
                    .map_err(HyperError::from);
            }
            return Err(HyperError::Network(format!(
                "GET {url}: HTTP {}",
                resp.status()
            )));
        }

        let mut body = Vec::new();
        resp.read_to_end(&mut body)?;
        let etag = resp
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        if let Some(parent) = cache_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = fs::write(
            &cache_file,
            serde_json::to_vec(&CachedResource {
                etag,
                fetched_at: now,
                body: body.clone(),
            })
            .unwrap_or_default(),
        );
        serde_json::from_slice(&body).map_err(HyperError::from)
    }

    /// Download a file to disk with a progress callback (bytes_done, total).
    pub fn download(
        &self,
        url: &str,
        dest: &Path,
        mut progress: impl FnMut(u64, Option<u64>),
    ) -> Result<u64> {
        Self::assert_allowed(url)?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut resp = self
            .client
            .get(url)
            .send()
            .map_err(|e| HyperError::Network(format!("GET {url}: {e}")))?;
        if !resp.status().is_success() {
            return Err(HyperError::Network(format!(
                "GET {url}: HTTP {}",
                resp.status()
            )));
        }
        let total = resp.content_length();
        let tmp = dest.with_extension(format!("part{}", std::process::id()));
        let mut file = fs::File::create(&tmp)?;
        let mut done = 0u64;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = resp.read(&mut buf)?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])?;
            done += n as u64;
            progress(done, total);
        }
        file.flush()?;
        fs::rename(&tmp, dest)?;
        Ok(done)
    }
}

pub fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

pub fn sha256_bytes(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

fn cache_file(key: &str) -> PathBuf {
    let safe: String = key
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '.' || c == '_' { c } else { '_' })
        .collect();
    paths::github_cache_dir().join(format!("{}.json", &safe[..safe.len().min(160)]))
}

fn read_cache(p: &Path) -> Option<CachedResource> {
    let bytes = fs::read(p).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn clear_cache(key_prefix: &str) -> Result<()> {
    let dir = paths::github_cache_dir();
    if dir.is_dir() {
        for e in fs::read_dir(&dir)? {
            let e = e?;
            if e.file_name().to_string_lossy().starts_with(key_prefix) {
                let _ = fs::remove_file(e.path());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_blocks_remote_http() {
        assert!(HttpClient::assert_allowed("https://example.com/x").is_ok());
        assert!(HttpClient::assert_allowed("http://127.0.0.1:8000/x").is_ok());
        assert!(HttpClient::assert_allowed("http://evil.com/x").is_err());
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}