use crate::error::{HyperError, Result};

const SERVICE: &str = "hyper-engine";
const TOKEN_USER: &str = "github-token";

/// Read the optional GitHub token from the OS credential store.
pub fn get_github_token() -> Option<String> {
    match get(SERVICE, TOKEN_USER) {
        Ok(Some(t)) if !t.is_empty() => Some(t),
        _ => None,
    }
}

pub fn set_github_token(token: &str) -> Result<()> {
    set(SERVICE, TOKEN_USER, token)
}

pub fn clear_github_token() -> Result<()> {
    if let Some(entry) = entry(SERVICE, TOKEN_USER) {
        let _ = entry.delete_credential();
    }
    Ok(())
}

fn entry(service: &str, user: &str) -> Option<keyring::Entry> {
    keyring::Entry::new(service, user).ok()
}

fn get(service: &str, user: &str) -> Result<Option<String>> {
    let Some(e) = entry(service, user) else {
        return Ok(None);
    };
    match e.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Ok(None),
    }
}

fn set(service: &str, user: &str, value: &str) -> Result<()> {
    let Some(e) = entry(service, user) else {
        return Err(HyperError::Message(
            "no OS credential store available".into(),
        ));
    };
    match e.set_password(value) {
        Ok(_) => Ok(()),
        Err(err) => Err(HyperError::Message(format!(
            "could not store token securely: {err}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_token() {
        // guard: only run when a store is actually available; skip otherwise
        if entry(SERVICE, "roundtrip-test").is_none() {
            return;
        }
        set(SERVICE, "roundtrip-test", "abc").unwrap();
        assert_eq!(get(SERVICE, "roundtrip-test").unwrap().as_deref(), Some("abc"));
        let _ = entry(SERVICE, "roundtrip-test").unwrap().delete_credential();
    }
}