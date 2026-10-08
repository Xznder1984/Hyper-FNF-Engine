use clap::Subcommand;
use hyper_core::download::HttpClient;
use hyper_pkg::{describe, parse_add_spec, ComingSoonRegistry, RegistryClient};

use crate::out;

#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(subcommand)]
    cmd: PkgCmd,
}

#[derive(Subcommand, Debug)]
enum PkgCmd {
    /// Add a mod from an explicit source: git <url> [rev] | release <url> [sha256]
    Add { args: Vec<String> },
    /// Search a package registry (not available yet - fails honestly)
    Search { query: String },
    /// Registry / source status
    Status,
}

pub fn run(args: Args) -> i32 {
    match run_inner(args) {
        Ok(code) => code,
        Err(e) => {
            out::fail(&e.to_string());
            1
        }
    }
}

fn run_inner(args: Args) -> hyper_pkg::Result<i32> {
    match args.cmd {
        PkgCmd::Add { args } => {
            let source = parse_add_spec(&args)?;
            out::status("source", &describe(&source));
            match &source {
                hyper_pkg::Source::Git { url, rev } => {
                    let git = "git";
                    let mut cmd = std::process::Command::new(git);
                    cmd.arg("clone").arg("--depth").arg("1");
                    if let Some(r) = rev {
                        cmd.arg("--branch").arg(r);
                    }
                    cmd.arg(url);
                    out::status("cloning", url);
                    match cmd.status() {
                        Ok(s) if s.success() => {
                            out::ok("mod cloned into the current directory");
                            Ok(0)
                        }
                        Ok(s) => {
                            out::fail(&format!("git clone exited with {}", s.code().unwrap_or(-1)));
                            Ok(1)
                        }
                        Err(e) => {
                            out::fail(&format!("could not run git: {e}"));
                            Ok(1)
                        }
                    }
                }
                hyper_pkg::Source::Release { url, sha256 } => {
                    if let Err(e) = HttpClient::assert_allowed(url) {
                        out::fail(&e.to_string());
                        return Ok(1);
                    }
                    let fname = url
                        .split('/')
                        .next_back()
                        .unwrap_or("mod-archive")
                        .to_string();
                    let dest = std::env::current_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("."))
                        .join(&fname);
                    out::status("fetch", url);
                    let http = HttpClient::new(None);
                    let got = http
                        .download(url, &dest, |d, t| {
                            if let Some(t) = t {
                                if t > 0 && d % (t / 40 + 1) < 1024 * 16 {
                                    eprint!("\r  {:.0}%", d as f64 / t as f64 * 100.0);
                                }
                            }
                        })
                        .map_err(|e| {
                            hyper_pkg::PkgError::Message(format!("download failed: {e}"))
                        })?;
                    out::ok(&format!("downloaded {got} bytes to {}", dest.display()));
                    if let Some(expected) = sha256 {
                        let actual = hyper_core::download::sha256_file(&dest).map_err(|e| {
                            hyper_pkg::PkgError::Message(format!("checksum failed: {e}"))
                        })?;
                        if !expected.eq_ignore_ascii_case(&actual) {
                            out::fail(&format!(
                                "sha256 mismatch: expected {expected}, got {actual}"
                            ));
                            return Ok(1);
                        }
                        out::ok("sha256 verified");
                    } else {
                        out::warn("no checksum supplied; verify the file yourself");
                    }
                    Ok(0)
                }
            }
        }
        PkgCmd::Search { query } => {
            let reg = ComingSoonRegistry;
            match reg.search(&query) {
                Ok(_) => unreachable!(),
                Err(e) => {
                    out::note(&e.to_string());
                    out::note(
                        "until a registry exists, add mods explicitly: hfe pkg add git <url>",
                    );
                    Ok(1)
                }
            }
        }
        PkgCmd::Status => {
            out::status(
                "registry",
                "coming soon (no fake registry, no placeholder packages)",
            );
            out::status(
                "sources",
                "git clone / release download today; manifest format designed",
            );
            Ok(0)
        }
    }
}
