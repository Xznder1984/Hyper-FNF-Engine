use std::io;

#[derive(Debug, thiserror::Error)]
pub enum HyperError {
    #[error("{0}")]
    Message(String),
    #[error("network: {0}")]
    Network(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("consent required: {0}")]
    Consent(String),
    #[error("cancelled")]
    Cancelled,
    #[error("invalid data: {0}")]
    Invalid(String),
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("zip: {0}")]
    Zip(String),
    #[error("checksum mismatch: expected {expected}, got {got}")]
    Checksum { expected: String, got: String },
    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),
}

pub type Result<T> = std::result::Result<T, HyperError>;

impl HyperError {
    /// Stable exit codes so scripts can rely on them.
    pub fn exit_code(&self) -> i32 {
        use HyperError::*;
        match self {
            Message(_) | Invalid(_) | Io(_) | Toml(_) | Json(_) => 1,
            Network(_) => 4,
            NotFound(_) => 3,
            Consent(_) => 5,
            Cancelled => 6,
            Zip(_) | Checksum { .. } | UnsupportedPlatform(_) => 1,
        }
    }
}

impl From<reqwest::Error> for HyperError {
    fn from(e: reqwest::Error) -> Self {
        HyperError::Network(e.to_string())
    }
}

impl From<zip::result::ZipError> for HyperError {
    fn from(e: zip::result::ZipError) -> Self {
        HyperError::Zip(e.to_string())
    }
}