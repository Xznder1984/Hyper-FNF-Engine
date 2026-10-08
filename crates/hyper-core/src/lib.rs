pub mod archive;
pub mod bench;
pub mod detect;
pub mod download;
pub mod engine;
pub mod error;
pub mod github;
pub mod keyring;
pub mod launch;
pub mod paths;
pub mod resolve;
pub mod sarahud;
pub mod settings;
pub mod store;
pub mod update;

pub use error::{HyperError, Result};
