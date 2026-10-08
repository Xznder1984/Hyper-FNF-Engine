use std::io::IsTerminal;
use std::path::PathBuf;

use clap::Subcommand;
use hyper_core::{
    engine::EngineLibrary, github::GitHub, keyring, resolve, sarahud, settings::Settings, store,
};

use crate::cmd::{confirm, printer};
use crate::out;

#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(subcommand)]
    cmd: AddCmd,
}

#[derive(Subcommand, Debug)]
enum AddCmd {
    /// Download and install an engine version (official source only)
    Engine {
        id: String,
        #[arg(default_value = "latest")]
        version: String,
        #[arg(long)]
        yes: bool,
    },
    /// Stage an existing mod folder into an installed engine
    Mod {
        path: PathBuf,
        #[arg(long)]
        engine: String,
        #[arg(long, default_value = "latest")]
        version: String,
    },
    /// Install SaraHUD for psych or fps-plus (opt-in, official source, credit shown)
    Sarahud {
        #[arg(value_parser = ["psych", "fps-plus"])]
        engine: String,
        #[arg(long)]
        yes: bool,
    },
    /// Set the base-game assets folder used by engines that need them
    Assets { path: PathBuf },
    /// Store a GitHub token securely (OS credential store) to lift API limits
    Token {
        #[command(subcommand)]
        action: TokenAction,
    },
}

#[derive(Subcommand, Debug)]
enum TokenAction {
    Set,
    Clear,
    Status,
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
    match args.cmd {
        AddCmd::Engine { id, version, yes } => add_engine(&id, &version, yes),
        AddCmd::Mod {
            path,
            engine,
            version,
        } => add_mod(&path, &engine, &version),
        AddCmd::Sarahud { engine, yes } => add_sarahud(&engine, yes),
        AddCmd::Assets { path } => add_assets(&path),
        AddCmd::Token { action } => token(action),
    }
}

fn add_engine(id: &str, version: &str, yes: bool) -> hyper_core::Result<i32> {
    let lib = EngineLibrary::load()?;
    let def = match lib.get(id) {
        Some(d) => d.clone(),
        None => {
            out::fail(&format!("unknown engine '{id}'; run 'hfe engines list'"));
            return Ok(1);
        }
    };
    let gh = GitHub::new();
    let repo = def.repo.as_deref().ok_or_else(|| {
        hyper_core::HyperError::Message(format!("engine '{}' has no release source", id))
    })?;
    let releases = gh.releases(repo)?;
    let tag = resolve::choose_tag(&releases, Some(version))?;

    let state = store::EngineState::load(id)?;
    if let Some(dir) = state.dir_for(&tag) {
        if dir.is_dir() {
            out::ok(&format!(
                "{id} {tag} is already installed at {}",
                dir.display()
            ));
            return Ok(0);
        }
    }

    let msg = format!(
        "download and install {id} {tag} from its official release ({})?",
        def.homepage.as_deref().unwrap_or(repo)
    );
    if !confirm(&msg, yes) {
        out::note("aborted");
        return Ok(0);
    }

    let release = releases
        .iter()
        .find(|r| r.tag == tag)
        .ok_or_else(|| hyper_core::HyperError::NotFound(format!("release {tag}")))?;
    let os = hyper_core::engine::os_key();
    let patterns = def.asset_candidates(&os);
    let asset = match gh.pick_asset(release, &os, &patterns)? {
        Some(a) => a,
        None => {
            out::fail(&format!("no {os} build found in release {tag}"));
            return Ok(1);
        }
    };
    out::status("fetch", &format!("{} ({})", asset.name, asset.url));
    let dest = hyper_core::paths::cache_dir()
        .join("downloads")
        .join(&asset.name);
    if !dest.exists() {
        gh.download_asset(&asset, &dest, asset.digest.as_deref(), printer())?;
    }
    let root = store::install_from_archive(&def, &tag, &dest, asset.digest.clone(), printer())?;
    out::ok(&format!("{id} {tag} installed at {}", root.display()));
    Ok(0)
}

fn add_mod(path: &std::path::Path, engine: &str, version: &str) -> hyper_core::Result<i32> {
    let lib = EngineLibrary::load()?;
    let def = match lib.get(engine) {
        Some(d) => d.clone(),
        None => {
            out::fail(&format!("unknown engine '{engine}'"));
            return Ok(1);
        }
    };
    let state = store::EngineState::load(engine)?;
    let dir = if version == "latest" {
        state.latest_dir()
    } else {
        state.dir_for(version)
    };
    let root = match dir {
        Some(d) if d.is_dir() => d,
        _ => {
            out::fail(&format!(
                "{engine} {version} is not installed; run 'hfe add engine {engine} {version}' first"
            ));
            return Ok(1);
        }
    };
    if !path.is_dir() {
        out::fail(&format!("mod folder not found: {}", path.display()));
        return Ok(1);
    }
    let staged = hyper_core::launch::stage_mod(&root, &def.mods_dir, path)?;
    out::ok(&format!(
        "mod '{staged}' staged into {}",
        root.join(&def.mods_dir).display()
    ));
    Ok(0)
}

fn add_sarahud(engine: &str, yes: bool) -> hyper_core::Result<i32> {
    let lib = EngineLibrary::load()?;
    let def = match lib.get(engine) {
        Some(d) => d.clone(),
        None => {
            out::fail(&format!("unknown engine '{engine}'"));
            return Ok(1);
        }
    };
    let Some(kind) = &def.sarahud else {
        out::fail(&format!("SaraHUD is not available for engine '{engine}'"));
        return Ok(1);
    };
    let state = store::EngineState::load(engine)?;
    let root = match state.versions.first().map(|v| v.dir.clone()) {
        Some(d) if d.is_dir() => d,
        _ => {
            out::fail(&format!(
                "install an engine first: 'hfe add engine {engine}'"
            ));
            return Ok(1);
        }
    };
    out::note(&format!(
        "{} ({}, {})",
        sarahud::CREDIT,
        sarahud::CREDIT_HOMEPAGE,
        sarahud::CREDIT_GAMEBANANA
    ));
    out::note("SaraHUD will be downloaded only from the author's official GitHub releases and set first in mod order, as its README requests.");
    if !confirm("install SaraHUD?", yes) {
        out::note("aborted");
        return Ok(0);
    }
    let gh = GitHub::new();
    let target = sarahud::install(&gh, kind, &root, &def.mods_dir, printer())?;
    let mods_dir = root.join(&def.mods_dir);
    sarahud::set_first_in_order(&mods_dir, def.order_file.as_deref(), sarahud::FOLDER)?;
    out::ok(&format!("SaraHUD installed at {}", target.display()));
    Ok(0)
}

fn add_assets(path: &std::path::Path) -> hyper_core::Result<i32> {
    let mut settings = Settings::load()?;
    if !path.is_dir() && !path.is_file() {
        out::fail(&format!("path not found: {}", path.display()));
        return Ok(1);
    }
    settings.game_assets = Some(path.to_string_lossy().to_string());
    settings.save()?;
    out::ok(&format!("game assets path set to {}", path.display()));
    Ok(0)
}

fn token(action: TokenAction) -> hyper_core::Result<i32> {
    match action {
        TokenAction::Status => {
            match keyring::get_github_token() {
                Some(_) => out::ok("a GitHub token is stored (OS credential store)"),
                None => out::note("no token stored; rate limit is 60 API requests/hour"),
            }
            Ok(0)
        }
        TokenAction::Clear => {
            keyring::clear_github_token()?;
            out::ok("token removed");
            Ok(0)
        }
        TokenAction::Set => {
            if std::io::stdin().is_terminal() {
                out::note("paste a GitHub token (fine-grained, read-only 'Contents: metadata' is enough):");
                match rpassword::read_password() {
                    Ok(t) if !t.trim().is_empty() => keyring::set_github_token(t.trim())?,
                    Ok(_) => {
                        out::fail("empty token");
                        return Ok(1);
                    }
                    Err(e) => {
                        out::fail(&format!("could not read token: {e}"));
                        return Ok(1);
                    }
                }
            } else {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                if line.trim().is_empty() {
                    out::fail("empty token");
                    return Ok(1);
                }
                keyring::set_github_token(line.trim())?;
            }
            out::ok("token stored securely (never written to files or logs)");
            Ok(0)
        }
    }
}
