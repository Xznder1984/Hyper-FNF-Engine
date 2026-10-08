use crate::error::{HyperError, Result};
use crate::github::Release;

/// Resolve which engine release tag to install.
///
/// - `requested` == "latest" or None -> newest non-prerelease (or newest if
///   the repo only ships prereleases).
/// - `requested` == some tag -> exact tag, verified to exist.
/// - `requested` == semver-ish prefix -> newest satisfying tag.
pub fn choose_tag(releases: &[Release], requested: Option<&str>) -> Result<String> {
    match requested {
        None | Some("latest") => {
            let stable = releases.iter().find(|r| !r.prerelease);
            match stable.or_else(|| releases.first()) {
                Some(r) => Ok(r.tag.clone()),
                None => Err(HyperError::NotFound(
                    "no releases found for this engine".into(),
                )),
            }
        }
        Some(tag) => {
            let t = tag.trim().trim_start_matches('v');
            if let Some(r) = releases.iter().find(|r| r.tag.trim_start_matches('v') == t) {
                return Ok(r.tag.clone());
            }
            // prefix match (semver-ish)
            let norm = |s: &str| s.trim_start_matches('v').to_lowercase();
            let mut best: Option<&Release> = None;
            for r in releases {
                let rt = norm(&r.tag);
                if rt == t || rt.starts_with(t) {
                    best = Some(match best {
                        None => r,
                        Some(b) => {
                            if norm(&r.tag) > norm(&b.tag) {
                                r
                            } else {
                                b
                            }
                        }
                    });
                }
            }
            match best {
                Some(r) => Ok(r.tag.clone()),
                None => Err(HyperError::NotFound(format!(
                    "no release matches '{tag}' (available: {})",
                    releases
                        .iter()
                        .take(8)
                        .map(|r| r.tag.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::ReleaseAsset;

    fn rel(tag: &str, pre: bool) -> Release {
        Release {
            tag: tag.into(),
            name: None,
            prerelease: pre,
            published_at: Some("2024-01-01T00:00:00Z".into()),
            assets: vec![ReleaseAsset {
                name: "x.zip".into(),
                url: "u".into(),
                size: None,
                digest: None,
            }],
        }
    }

    #[test]
    fn prefers_stable() {
        let releases = vec![rel("9.0.0-pre.1", true), rel("1.0.4", false)];
        assert_eq!(choose_tag(&releases, None).unwrap(), "1.0.4");
        assert_eq!(choose_tag(&releases, Some("latest")).unwrap(), "1.0.4");
    }

    #[test]
    fn fallback_to_prerelease_when_only_pre() {
        let releases = vec![rel("9.0.0-pre.1", true)];
        assert_eq!(choose_tag(&releases, None).unwrap(), "9.0.0-pre.1");
    }

    #[test]
    fn exact_and_prefix() {
        let releases = vec![rel("2.0", false), rel("2.1", false), rel("1.1", false)];
        assert_eq!(choose_tag(&releases, Some("2.0")).unwrap(), "2.0");
        assert_eq!(choose_tag(&releases, Some("2")).unwrap(), "2.1");
        assert_eq!(choose_tag(&releases, Some("v1.1")).unwrap(), "1.1");
        assert!(choose_tag(&releases, Some("nope")).is_err());
    }
}
