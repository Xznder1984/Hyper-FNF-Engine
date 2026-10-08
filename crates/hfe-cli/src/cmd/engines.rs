use clap::Subcommand;
use hyper_core::{download::clear_cache, engine::EngineLibrary, github::GitHub, resolve};

use crate::out;

#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(subcommand)]
    cmd: EngCmd,
}

#[derive(Subcommand, Debug)]
enum EngCmd {
    /// List engine definitions (no network)
    List,
    /// Show one engine definition
    Show { id: String },
    /// List available release tags (network; cached)
    Tags {
        id: String,
        #[arg(long)]
        include_pre: bool,
    },
    /// Clear the GitHub/GameBanana response cache
    CacheClear,
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
    let lib = EngineLibrary::load()?;
    match args.cmd {
        EngCmd::List => {
            println!(
                "{:<16} {:<28} {}",
                out::bold("id"),
                out::bold("name"),
                out::bold("release source")
            );
            for d in &lib.defs {
                let src = d.repo.as_deref().unwrap_or("direct URL");
                println!("{:<16} {:<28} {}", out::cyan(&d.id), d.name, src);
            }
            Ok(0)
        }
        EngCmd::Show { id } => {
            let d = lib
                .get(&id)
                .ok_or_else(|| hyper_core::HyperError::NotFound(format!("engine '{id}'")))?;
            println!("id          : {}", d.id);
            println!("name        : {}", d.name);
            println!(
                "license     : {}",
                d.license.as_deref().unwrap_or("unknown")
            );
            println!("homepage    : {}", d.homepage.as_deref().unwrap_or("-"));
            println!("repo        : {}", d.repo.as_deref().unwrap_or("-"));
            println!("mods_dir    : {}", d.mods_dir);
            println!("order file  : {}", d.order_file.as_deref().unwrap_or("-"));
            println!("mod meta    : {}", d.mod_metadata.join(", "));
            println!("sarahud     : {}", d.sarahud.as_deref().unwrap_or("no"));
            println!("scripts warn: {}", d.script_warning);
            if !d.notes.is_empty() {
                println!("notes       :");
                for n in &d.notes {
                    println!("  - {n}");
                }
            }
            Ok(0)
        }
        EngCmd::Tags { id, include_pre } => {
            let d = lib
                .get(&id)
                .ok_or_else(|| hyper_core::HyperError::NotFound(format!("engine '{id}'")))?;
            let repo = d.repo.as_deref().ok_or_else(|| {
                hyper_core::HyperError::Message(format!("engine '{id}' has no GitHub source"))
            })?;
            let gh = GitHub::new();
            let releases = gh.releases(repo)?;
            println!("{} tags (cached, GitHub API):", d.name);
            for r in releases.iter().take(20) {
                let pre = if r.prerelease { " [pre-release]" } else { "" };
                println!(
                    "  {:<20} {}{}",
                    out::green(&r.tag),
                    if r.assets.is_empty() {
                        " (no assets)"
                    } else {
                        ""
                    },
                    pre
                );
            }
            let latest = resolve::choose_tag(&releases, Some("latest"))?;
            println!("recommended: {latest}");
            let _ = include_pre;
            Ok(0)
        }
        EngCmd::CacheClear => {
            clear_cache("releases-")?;
            clear_cache("release-")?;
            out::ok("GitHub cache cleared");
            Ok(0)
        }
    }
}
