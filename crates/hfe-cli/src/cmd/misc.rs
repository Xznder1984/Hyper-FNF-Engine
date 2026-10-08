use hyper_core::{launch::managed_install, paths};

use crate::out;

pub fn version() -> i32 {
    println!("hyper-engine {}", env!("CARGO_PKG_VERSION"));
    println!(
        "bin        : {}",
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".into())
    );
    println!("data dir   : {}", paths::data_dir().display());
    println!("cache dir  : {}", paths::cache_dir().display());
    println!("config dir : {}", paths::config_dir().display());
    if let Some(m) = managed_install() {
        out::note(&format!("managed install: {m}"));
    }
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    println!("platform   : {os}-{arch}");
    println!("license    : Apache-2.0 (Hyper Engine code)");
    println!("            unofficial; not affiliated with The Funkin' Crew Inc.");
    0
}
