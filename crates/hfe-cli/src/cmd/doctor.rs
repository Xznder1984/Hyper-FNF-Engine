use hyper_core::{
    engine::EngineLibrary, paths, settings::Settings, store,
};

use crate::out;

struct Check {
    name: String,
    ok: bool,
    detail: String,
}

pub fn run() -> i32 {
    let mut checks: Vec<Check> = Vec::new();
    let mut failed = 0;

    // 1. data directories writable
    for (name, dir) in [
        ("data dir", paths::data_dir()),
        ("cache dir", paths::cache_dir()),
        ("config dir", paths::config_dir()),
    ] {
        let created = std::fs::create_dir_all(&dir).is_ok();
        checks.push(Check {
            name: name.into(),
            ok: created,
            detail: dir.display().to_string(),
        });
    }
    // test writes
    let probe = paths::data_dir().join(".write-probe");
    let writable = std::fs::write(&probe, b"ok").is_ok()
        && std::fs::remove_file(&probe).is_ok();
    checks.push(Check {
        name: "dirs writable".into(),
        ok: writable,
        detail: "probe file create+delete".into(),
    });

    // 2. settings readable
    match Settings::load() {
        Ok(_) => checks.push(Check {
            name: "settings".into(),
            ok: true,
            detail: "readable".into(),
        }),
        Err(e) => checks.push(Check {
            name: "settings".into(),
            ok: false,
            detail: e.to_string(),
        }),
    }

    // 3. engine definitions
    match EngineLibrary::load() {
        Ok(lib) => checks.push(Check {
            name: "engine defs".into(),
            ok: true,
            detail: format!("{} definitions loaded", lib.defs.len()),
        }),
        Err(e) => checks.push(Check {
            name: "engine defs".into(),
            ok: false,
            detail: e.to_string(),
        }),
    }

    // 4. installed engine integrity
    match store::installed_engines() {
        Ok(states) => {
            for s in states {
                let missing = s
                    .versions
                    .iter()
                    .filter(|v| !v.dir.is_dir())
                    .count();
                if missing == 0 {
                    checks.push(Check {
                        name: format!("engine {}", s.id),
                        ok: true,
                        detail: format!("{} version(s) intact", s.versions.len()),
                    });
                } else {
                    failed += 1;
                    checks.push(Check {
                        name: format!("engine {}", s.id),
                        ok: false,
                        detail: format!("{missing} recorded version(s) missing on disk"),
                    });
                }
            }
        }
        Err(e) => checks.push(Check {
            name: "engine store".into(),
            ok: false,
            detail: e.to_string(),
        }),
    }

    // 5. managed install (affects updates)
    if let Some(mgr) = hyper_core::launch::managed_install() {
        checks.push(Check {
            name: "install type".into(),
            ok: true,
            detail: format!("managed: {mgr}; self-update disabled"),
        });
    }

    // 6. GitHub token presence (not the token itself)
    match hyper_core::keyring::get_github_token() {
        Some(_) => checks.push(Check {
            name: "github token".into(),
            ok: true,
            detail: "present (OS credential store)".to_string(),
        }),
        None => checks.push(Check {
            name: "github token".into(),
            ok: true,
            detail: "absent; API limit is 60 req/hour (fine for normal use)".to_string(),
        }),
    }

    // 7. git available (needed for `pkg add git`)
    let has_git = std::process::Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    checks.push(Check {
        name: "git".into(),
        ok: has_git,
        detail: if has_git { "available" } else { "missing; 'hfe pkg add git' will not work" }.into(),
    });

    // report
    println!("{}", out::bold("hyper doctor"));
    for c in &checks {
        let mark = if c.ok {
            out::green("pass")
        } else {
            out::red("FAIL")
        };
        println!("  {mark}  {:<14} {}", c.name, out::dim(&c.detail));
        if !c.ok {
            failed += 1;
        }
    }

    if failed > 0 {
        out::warn(&format!("{failed} check(s) failed"));
        return 1;
    }
    out::ok("all checks passed");
    0
}