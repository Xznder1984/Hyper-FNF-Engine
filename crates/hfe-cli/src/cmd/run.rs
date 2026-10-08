use std::path::PathBuf;

use hyper_core::{
    detect::{self, mod_key},
    engine::EngineLibrary,
    github::{GitHub, Release},
    launch,
    paths, resolve, sarahud, settings::Settings, store,
};

use crate::out;
use crate::cmd::printer;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Mod folder to run (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Force an engine id (skips/overrides detection)
    #[arg(long)]
    engine: Option<String>,

    /// Pin an engine version tag (e.g. "1.0.4", "2", "latest")
    #[arg(long)]
    version: Option<String>,

    /// Launcher-level process priority: low|normal|high|realtime
    #[arg(long, value_parser = clap::value_parser!(launch::Priority))]
    priority: Option<launch::Priority>,

    /// Plan everything (detect/resolve/install) but do not launch
    #[arg(long)]
    dry_run: bool,

    /// Do not ask for confirmation
    #[arg(long)]
    yes: bool,
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
    let lib = EngineLibrary::load()?;
    let mod_dir = if args.path.as_os_str() == "." {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else {
        args.path.clone()
    };
    if !mod_dir.is_dir() {
        out::fail(&format!("mod folder not found: {}", mod_dir.display()));
        return Ok(1);
    }

    // 1. engine resolution
    let engine_id = match &args.engine {
        Some(id) if lib.get(id).is_some() => id.clone(),
        Some(id) => {
            out::fail(&format!("unknown engine '{id}'; see 'hfe engines list'"));
            return Ok(1);
        }
        _ => {
            let det = detect::detect(&mod_dir, &lib, &settings);
            match det.engine_id {
                Some(id) => {
                    out::status("detected", &format!("{id} ({})", det.confidence.as_str()));
                    if det.confidence == detect::Confidence::Low {
                        out::warn("low confidence; pass --engine <id> to override");
                    }
                    for ev in &det.evidence {
                        out::note(ev);
                    }
                    id
                }
                None => {
                    out::fail("could not detect the engine for this mod");
                    out::note("run 'hfe engines list' and retry with --engine <id>");
                    return Ok(1);
                }
            }
        }
    };
    let def = lib.get(&engine_id).unwrap().clone();

    if def.script_warning {
        out::warn("this engine runs mod scripts (they can execute code in-game).");
    }

    // 2. version resolution (pinned > flag > latest)
    let key = mod_key(&mod_dir)?;
    let requested = args
        .version
        .clone()
        .or_else(|| settings.pinned_version(&key))
        .or_else(|| settings.profile_for(&key).engine_version.clone());

    let state = store::EngineState::load(&def.id)?;
    let gh = GitHub::new();
    let mut releases: Vec<Release> = Vec::new();
    let tag = match &def.repo {
        Some(repo) => match gh.releases(repo) {
            Ok(r) => {
                releases = r;
                resolve::choose_tag(&releases, requested.as_deref())?
            }
            Err(e) => {
                out::warn(&format!("could not list releases for {repo}: {e}"));
                match state.versions.first() {
                    Some(v) => v.version.clone(),
                    None => {
                        out::fail("no releases reachable and nothing is installed");
                        return Ok(1);
                    }
                }
            }
        },
        None => {
            out::fail(&format!("engine '{}' has no release source", def.id));
            return Ok(1);
        }
    };
    out::status("version", &tag);

    // 3. install if needed
    let engine_root = match state.dir_for(&tag) {
        Some(dir) if dir.is_dir() => dir,
        _ => {
            if args.dry_run {
                out::note(&format!(
                    "would download and install {engine_id} {tag} from its official release"
                ));
                return Ok(0);
            }
            if releases.is_empty() {
                out::fail(&format!(
                    "{} {tag} is not installed and no release list is available (offline)",
                    def.id
                ));
                return Ok(1);
            }
            let release = releases.iter().find(|r| r.tag == tag).unwrap();
            let os = hyper_core::engine::os_key();
            let patterns = def.asset_candidates(&os);
            let asset = match gh.pick_asset(release, &os, &patterns)? {
                Some(a) => a,
                None => {
                    out::fail(&format!(
                        "no {os} build found in release {tag} (assets: {})",
                        release
                            .assets
                            .iter()
                            .map(|a| a.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                    return Ok(1);
                }
            };
            out::status("fetch", &format!("{engine_id} {tag} ({})", asset.name));
            let dest = paths::cache_dir().join("downloads").join(&asset.name);
            let digest: Option<String> = asset.digest.clone();
            if !dest.exists() || digest.is_none() {
                out::note(&format!(
                    "downloading {} ({:.1} MiB)",
                    asset.name,
                    asset.size.unwrap_or(0) as f64 / 1048576.0
                ));
                let sha = gh.download_asset(&asset, &dest, digest.as_deref(), printer())?;
                out::ok(&format!("downloaded (sha256 {}", &sha[..12]));
            }
            let installed = store::install_from_archive(&def, &tag, &dest, digest, printer())?;
            out::ok(&format!("installed to {}", installed.display()));
            installed
        }
    };

    // 4. game assets, when the engine needs them from a user-provided copy
    if def.needs_base_assets {
        if let Some(game) = &settings.game_assets {
            ensure_assets(&engine_root, game)?;
        } else {
            out::warn(&format!(
                "{engine_id} needs base game assets; set them with 'hfe add assets <path>'"
            ));
        }
    }

    // 5. SaraHUD support (opt-in only)
    if settings.sarahud.enabled {
        if let Some(kind) = &def.sarahud {
            let mods_dir = engine_root.join(&def.mods_dir);
            if !sarahud::is_installed(&mods_dir, sarahud::FOLDER) {
                out::status("sarahud", "installing (opt-in, official source)");
                sarahud::install(&gh, kind, &engine_root, &def.mods_dir, printer())?;
                sarahud::set_first_in_order(&mods_dir, def.order_file.as_deref(), sarahud::FOLDER)?;
                out::ok("SaraHUD installed and set first in mod order");
            }
        } else {
            out::note(&format!("{engine_id} does not support SaraHUD; skipping"));
        }
    }

    // 6. launch
    let exe = match launch::find_executable(&engine_root) {
        Some(e) => e,
        None => {
            out::fail(&format!(
                "no engine executable found under {}",
                engine_root.display()
            ));
            return Ok(1);
        }
    };

    if args.dry_run {
        out::status("would launch", &exe.display().to_string());
        out::note("dry-run: engine fetched/verified, not launched");
        return Ok(0);
    }

    let staged = launch::stage_mod(&engine_root, &def.mods_dir, &mod_dir)?;
    out::status("staged", &format!("mod '{}' -> {}/", staged, def.mods_dir));

    // record the engine choice so next time detection is instant & decisive
    settings.set_engine_override(&key, engine_id.clone());
    if let Some(v) = &args.version {
        settings.pinned.insert(key.clone(), v.clone());
    }
    let _ = settings.save();

    let opts = launch::LaunchOptions {
        priority: args.priority,
        extra_args: vec![],
        env: vec![],
    };
    out::status("launch", &exe.display().to_string());
    let child = launch::launch(&engine_root, &exe, &opts)?;
    println!(
        "{} pid {} - Hyper Engine is done; the game keeps running.",
        out::green("running"),
        child.id()
    );
    Ok(0)
}

fn ensure_assets(engine_root: &std::path::Path, game: &str) -> hyper_core::Result<()> {
    let src = std::path::Path::new(game);
    let assets_src = if src.is_file() {
        src.parent()
    } else {
        Some(src)
    }
    .unwrap_or(src);
    let assets_src = if assets_src.join("assets").is_dir() {
        assets_src.join("assets")
    } else {
        assets_src.to_path_buf()
    };
    let dst = engine_root.join("assets");
    if dst.exists() {
        return Ok(());
    }
    if !assets_src.is_dir() {
        out::warn(&format!("no assets/ found in {}", game));
        return Ok(());
    }
    out::status("assets", &format!("copying {} -> {}", assets_src.display(), dst.display()));
    copy_dir(&assets_src, &dst)?;
    Ok(())
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> hyper_core::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &to)?;
        } else {
            std::fs::copy(e.path(), &to)?;
        }
    }
    Ok(())
}