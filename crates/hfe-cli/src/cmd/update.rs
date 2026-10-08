use clap::Subcommand;
use hyper_core::{
    download::HttpClient, launch::managed_install, paths, settings::Settings, update,
};

use crate::cmd::printer;
use crate::out;

#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(subcommand)]
    cmd: UpdateCmd,

    /// Override the update manifest URL (used for tests against a fake server)
    #[arg(long, global = true, default_value = update::DEFAULT_MANIFEST_URL)]
    manifest_url: String,

    /// Skip the manual steps and perform the action without asking
    #[arg(long, global = true)]
    yes: bool,
}

#[derive(Subcommand, Debug)]
enum UpdateCmd {
    /// Check for a launcher update (respects settings + managed installs)
    Check,
    /// Fetch + verify + install the newest version
    Install,
    /// Restore the previous launcher binary
    Rollback,
    /// Show installed/manifest/rollback status
    Status,
    /// Enable major-version auto updates (off by default)
    EnableMajor,
    /// Disable automatic update checks entirely
    Disable,
}

pub fn run(args: Args) -> i32 {
    match run_inner(args) {
        Ok(code) => code,
        Err(e) => {
            out::fail(&e.to_string());
            e.exit_code()
        }
    }
}

fn run_inner(args: Args) -> hyper_core::Result<i32> {
    let mut settings = Settings::load()?;
    match args.cmd {
        UpdateCmd::Check => {
            if let Some(mgr) = managed_install() {
                out::note(&format!(
                    "managed install detected ({mgr}); self-update is disabled — update via the package manager"
                ));
                return Ok(0);
            }
            let http = HttpClient::new(None);
            match update::check(&settings, &http, &args.manifest_url) {
                Ok(Some(up)) => {
                    match &up.artifact {
                        Some(a) => out::status("update", &format!("{} -> {} ({})", update::read_installed_version().unwrap_or_else(|| env!("CARGO_PKG_VERSION").into()), up.manifest.version, a.url)),
                        None => out::status("update", &up.reason),
                    }
                    if let Some(n) = &up.manifest.notes {
                        out::note(n);
                    }
                    if up.manifest.major && !settings.update.allow_major {
                        out::warn("this is a major release; major updates require explicit opt-in ('hfe update enablemajor')");
                    }
                }
                Ok(None) => out::ok("no update available"),
                Err(e @ hyper_core::HyperError::Message(_)) => {
                    out::warn(&e.to_string());
                }
                Err(e) if e.to_string().contains("404") => {
                    out::warn("no update channel found for this build (pre-release/dev build)");
                }
                Err(e) => return Err(e),
            }
            Ok(0)
        }
        UpdateCmd::Install => {
            if let Some(mgr) = managed_install() {
                out::fail(&format!(
                    "managed install detected ({mgr}); don't self-update — use the package manager"
                ));
                return Ok(1);
            }
            let http = HttpClient::new(None);
            let up = match update::check(&settings, &http, &args.manifest_url)? {
                Some(u) => u,
                None => {
                    out::ok("already up to date");
                    return Ok(0);
                }
            };
            let artifact = match &up.artifact {
                Some(a) => a.clone(),
                None => {
                    out::fail(&format!("no artifact for this platform: {}", up.reason));
                    return Ok(1);
                }
            };
            out::status("update", &format!("{} -> {}", update::read_installed_version().unwrap_or_else(|| env!("CARGO_PKG_VERSION").into()), up.manifest.version));
            if !crate::cmd::confirm("install this update?", args.yes) {
                out::note("aborted");
                return Ok(0);
            }
            let candidate = update::fetch_and_verify(&artifact, &http, printer())?;
            out::ok(&format!("verified {}", candidate.display()));
            match update::install_verified(&candidate) {
                Ok(()) => {
                    update::record_installed(&up.manifest.version)?;
                    out::ok("installed. The next launch uses the new binary.");
                    Ok(0)
                }
                Err(e) => {
                    out::fail(&e.to_string());
                    Ok(1)
                }
            }
        }
        UpdateCmd::Rollback => {
            match update::rollback() {
                Ok(b) => {
                    out::ok(&format!("restored previous binary from {}", b.display()));
                    Ok(0)
                }
                Err(e) => {
                    out::fail(&e.to_string());
                    Ok(e.exit_code())
                }
            }
        }
        UpdateCmd::Status => {
            println!("installed : {}", update::read_installed_version().unwrap_or_else(|| env!("CARGO_PKG_VERSION").into()));
            let m = paths::launcher_manifest_path();
            println!("backups   : {}", paths::rollback_dir().display());
            println!("managed   : {}", managed_install().unwrap_or_else(|| "no".into()));
            println!("updates   : {}", if settings.update.enabled { "enabled" } else { "disabled" });
            println!("auto-check: {}", settings.update.auto_check);
            println!("majors    : {}", settings.update.allow_major);
            let _ = m;
            Ok(0)
        }
        UpdateCmd::EnableMajor => {
            settings.update.allow_major = true;
            settings.save()?;
            out::ok("major updates enabled (still only installed when you run 'hfe update install')");
            Ok(0)
        }
        UpdateCmd::Disable => {
            settings.update.enabled = false;
            settings.save()?;
            out::ok("update checks disabled");
            Ok(0)
        }
    }
}