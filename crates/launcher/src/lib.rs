use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use hyper_core::detect;
use hyper_core::engine::EngineLibrary;
use hyper_core::github::{GitHub, Release};
use hyper_core::settings::Settings;
use hyper_core::{launch, paths, resolve, sarahud, store};

/// Serializable summary of a detection. Mirrored in haxe/DetectDto.hx.
#[derive(Serialize)]
struct ModDetect {
    engine_id: Option<String>,
    confidence: String,
    candidates: Vec<CandidateDto>,
    evidence: Vec<String>,
}

#[derive(Serialize)]
struct CandidateDto {
    engine_id: String,
    score: i32,
    evidence: Vec<String>,
}

/// Serializable engine list entry. Mirrored in haxe/EngineDto.hx.
#[derive(Serialize)]
struct EngineInfo {
    id: String,
    name: String,
    license: Option<String>,
    homepage: Option<String>,
    repo: Option<String>,
    notes: Vec<String>,
    installed: Vec<String>,
    sarahud: Option<String>,
}

#[derive(Serialize)]
struct LaunchOutcome {
    pid: u32,
    exe: String,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Detect which engine a mod folder targets.
#[tauri::command]
async fn detect_mod(mod_dir: String) -> Result<ModDetect, String> {
    run_blocking(move || {
        let lib = EngineLibrary::load().map_err(|e| e.to_string())?;
        let settings = Settings::load().map_err(|e| e.to_string())?;
        let dir = PathBuf::from(&mod_dir);
        if !dir.is_dir() {
            return Err(format!("not a folder: {mod_dir}"));
        }
        let d = detect::detect(&dir, &lib, &settings);
        Ok(ModDetect {
            engine_id: d.engine_id,
            confidence: d.confidence.as_str().to_string(),
            candidates: d
                .candidates
                .into_iter()
                .map(|c| CandidateDto {
                    engine_id: c.engine_id,
                    score: c.score,
                    evidence: c.evidence,
                })
                .collect(),
            evidence: d.evidence,
        })
    })
    .await
}

/// List supported engines plus their locally installed versions.
#[tauri::command]
async fn engines() -> Result<Vec<EngineInfo>, String> {
    run_blocking(move || {
        let lib = EngineLibrary::load().map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for def in &lib.defs {
            let installed = store::EngineState::load(&def.id)
                .map(|s| s.versions.iter().map(|v| v.version.clone()).collect())
                .unwrap_or_default();
            out.push(EngineInfo {
                id: def.id.clone(),
                name: def.name.clone(),
                license: def.license.clone(),
                homepage: def.homepage.clone(),
                repo: def.repo.clone(),
                notes: def.notes.clone(),
                installed,
                sarahud: def.sarahud.clone(),
            });
        }
        Ok(out)
    })
    .await
}

/// Current settings (overrides, pins, game assets, SaraHUD) as JSON for the UI.
#[tauri::command]
async fn settings_get() -> Result<serde_json::Value, String> {
    run_blocking(move || {
        let s = Settings::load().map_err(|e| e.to_string())?;
        serde_json::to_value(&s).map_err(|e| e.to_string())
    })
    .await
}

/// Remember the detected engine for a mod so launches are instant + decisive.
#[tauri::command]
async fn remember_engine(mod_dir: String, engine: String) -> Result<(), String> {
    run_blocking(move || {
        let key = detect::mod_key(&PathBuf::from(&mod_dir)).map_err(|e| e.to_string())?;
        let mut s = Settings::load().map_err(|e| e.to_string())?;
        s.set_engine_override(&key, engine);
        s.save().map_err(|e| e.to_string())
    })
    .await
}

/// Forget the recorded engine choice for a mod.
#[tauri::command]
async fn forget_engine(mod_dir: String) -> Result<(), String> {
    run_blocking(move || {
        let key = detect::mod_key(&PathBuf::from(&mod_dir)).map_err(|e| e.to_string())?;
        let mut s = Settings::load().map_err(|e| e.to_string())?;
        s.overrides.remove(&key);
        s.pinned.remove(&key);
        s.save().map_err(|e| e.to_string())
    })
    .await
}

/// Set the path to a local copy of the base game (for engines that need it).
#[tauri::command]
async fn set_game_assets(path: String) -> Result<(), String> {
    run_blocking(move || {
        let mut s = Settings::load().map_err(|e| e.to_string())?;
        s.game_assets = Some(path);
        s.save().map_err(|e| e.to_string())
    })
    .await
}

/// Opt in/out of SaraHUD auto-install (official source only, set first in mod order).
#[tauri::command]
async fn sarahud_enabled(enabled: bool) -> Result<(), String> {
    run_blocking(move || {
        let mut s = Settings::load().map_err(|e| e.to_string())?;
        s.sarahud.enabled = enabled;
        s.save().map_err(|e| e.to_string())
    })
    .await
}

/// Full flow: detect -> resolve -> install if needed -> stage -> launch.
/// Mirrors `hfe run`. Progress is streamed to the frontend as events.
#[tauri::command]
async fn launch_mod(
    app: AppHandle,
    mod_dir: String,
    requested: Option<String>,
    priority: Option<String>,
) -> Result<LaunchOutcome, String> {
    run_blocking(move || launch_inner(&app, mod_dir, requested, priority)).await
}

/// Open a URL in the system browser (https only).
#[tauri::command]
async fn open_external(url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("only https:// links are allowed".into());
    }
    run_blocking(move || open::that_detached(&url).map_err(|e| e.to_string())).await
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run a blocking (CPU / IO heavy) body off the main thread.
async fn run_blocking<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    match tauri::async_runtime::spawn_blocking(f).await {
        Ok(r) => r,
        Err(e) => Err(format!("worker panicked: {e}")),
    }
}

fn progress(app: &AppHandle, msg: &str) {
    let _ = app.emit("hyper://progress", msg.to_string());
}

fn launch_inner(
    app: &AppHandle,
    mod_dir: String,
    requested: Option<String>,
    priority: Option<String>,
) -> Result<LaunchOutcome, String> {
    let lib = EngineLibrary::load().map_err(|e| e.to_string())?;
    let mut settings = Settings::load().map_err(|e| e.to_string())?;
    let dir = PathBuf::from(&mod_dir);
    if !dir.is_dir() {
        return Err(format!("mod folder not found: {}", dir.display()));
    }

    // 1. engine resolution (recorded override first, then detection)
    let key = detect::mod_key(&dir).map_err(|e| e.to_string())?;
    let engine_id = match settings.engine_override(&key) {
        Some(id) if lib.get(&id).is_some() => id,
        _ => {
            let d = detect::detect(&dir, &lib, &settings);
            let id = d.engine_id.ok_or_else(|| {
                "could not detect the engine for this mod; pick one from the list".to_string()
            })?;
            progress(
                app,
                &format!("detected {} ({})", id, d.confidence.as_str()),
            );
            for ev in &d.evidence {
                progress(app, ev);
            }
            id
        }
    };
    let def = lib
        .get(&engine_id)
        .ok_or_else(|| format!("unknown engine '{engine_id}'"))?
        .clone();

    // 2. version resolution: flag > pinned > profile
    let requested = requested
        .or_else(|| settings.pinned_version(&key))
        .or_else(|| settings.profile_for(&key).engine_version.clone());

    let state = store::EngineState::load(&def.id).map_err(|e| e.to_string())?;
    let gh = GitHub::new();
    let mut releases: Vec<Release> = Vec::new();
    let repo = def
        .repo
        .clone()
        .ok_or_else(|| format!("engine '{}' has no release source", def.id))?;
    let tag = match gh.releases(&repo) {
        Ok(r) => {
            releases = r;
            resolve::choose_tag(&releases, requested.as_deref()).map_err(|e| e.to_string())?
        }
        Err(e) => {
            progress(
                app,
                &format!("could not list releases ({e}); using installed version"),
            );
            match state.versions.first() {
                Some(v) => v.version.clone(),
                None => return Err("no releases reachable and nothing is installed".into()),
            }
        }
    };
    progress(app, &format!("version {tag}"));

    // 3. install if needed
    let engine_root = match state.dir_for(&tag) {
        Some(d) if d.is_dir() => d,
        _ => {
            if releases.is_empty() {
                return Err(format!(
                    "{} {tag} is not installed and no release list is available (offline)",
                    def.id
                ));
            }
            let release = releases
                .iter()
                .find(|r| r.tag == tag)
                .ok_or_else(|| format!("release {tag} not found in the list"))?;
            let os = hyper_core::engine::os_key();
            let asset = gh
                .pick_asset(release, &os, &def.asset_candidates(&os))
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("no {os} build found in release {tag}"))?;
            progress(app, &format!("fetching {} ({})", def.id, asset.name));
            let dest = paths::cache_dir().join("downloads").join(&asset.name);
            let digest: Option<String> = asset.digest.clone();
            if !dest.exists() || digest.is_none() {
                gh.download_asset(&asset, &dest, digest.as_deref(), |done, total| {
                    progress(app, &format!("downloading {done} / {total:?} bytes"));
                })
                .map_err(|e| e.to_string())?;
            }
            store::install_from_archive(&def, &tag, &dest, digest, |done, total| {
                progress(app, &format!("extracting {done} / {total:?} bytes"));
            })
            .map_err(|e| e.to_string())?
        }
    };

    // 4. SaraHUD (opt-in only)
    if settings.sarahud.enabled {
        if let Some(kind) = &def.sarahud {
            let mods_dir = engine_root.join(&def.mods_dir);
            if !sarahud::is_installed(&mods_dir, sarahud::FOLDER) {
                progress(app, "installing SaraHUD (opt-in)");
                sarahud::install(&gh, kind, &engine_root, &def.mods_dir, |done, total| {
                    progress(app, &format!("sarahud {done} / {total:?} bytes"));
                })
                .map_err(|e| e.to_string())?;
                sarahud::set_first_in_order(&mods_dir, def.order_file.as_deref(), sarahud::FOLDER)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    // 5. stage + launch
    let exe = launch::find_executable(&engine_root)
        .ok_or_else(|| format!("no engine executable found under {}", engine_root.display()))?;
    progress(app, &format!("staging mod into {}/", def.mods_dir));
    launch::stage_mod(&engine_root, &def.mods_dir, &dir).map_err(|e| e.to_string())?;

    // remember the choice for next time
    settings.set_engine_override(&key, engine_id.clone());
    let _ = settings.save();

    let prio = priority.as_deref().and_then(launch::Priority::parse);
    let opts = launch::LaunchOptions {
        priority: prio,
        extra_args: vec![],
        env: vec![],
    };
    let child = launch::launch(&engine_root, &exe, &opts).map_err(|e| e.to_string())?;
    progress(app, &format!("launched pid {}", child.id()));
    Ok(LaunchOutcome {
        pid: child.id(),
        exe: exe.display().to_string(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            detect_mod,
            engines,
            settings_get,
            remember_engine,
            forget_engine,
            set_game_assets,
            sarahud_enabled,
            launch_mod,
            open_external
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hyper Engine");
}