use clap::Subcommand;
use hyper_core::{detect, engine::EngineLibrary, launch, settings::Settings, store};

use crate::out;

#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(subcommand)]
    cmd: ListCmd,
}

#[derive(Subcommand, Debug)]
enum ListCmd {
    /// Installed engines and versions
    Engines,
    /// Detect the engine for a mod folder (read-only)
    Mods { path: std::path::PathBuf },
    /// Both of the above
    All,
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
        ListCmd::Engines => list_engines(),
        ListCmd::Mods { path } => detect_mod(&path),
        ListCmd::All => {
            list_engines()?;
            println!();
            detect_mod(&std::env::current_dir().unwrap_or_else(|_| ".".into()))
        }
    }
}

fn list_engines() -> hyper_core::Result<i32> {
    let states = store::installed_engines()?;
    let lib = EngineLibrary::load()?;
    if states.is_empty() {
        out::note("no engines installed yet. Try: hfe add engine psych");
        print_defs(&lib);
        return Ok(0);
    }
    println!("{}", out::bold("installed engines"));
    for s in &states {
        let def = lib.get(&s.id);
        let name = def.map(|d| d.name.as_str()).unwrap_or(&s.id);
        println!("  {}  {}", out::cyan(&s.id), name);
        for v in &s.versions {
            let exe = launch::find_executable(&v.dir)
                .map(|p| {
                    p.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                })
                .unwrap_or_else(|| "?".into());
            println!(
                "    {}  {}  {}  exe: {}",
                out::green(&v.version),
                out::dim(&v.dir.display().to_string()),
                out::dim(&human_size(dir_size(&v.dir))),
                exe
            );
        }
    }
    Ok(0)
}

fn print_defs(lib: &EngineLibrary) {
    println!("{}", out::bold("available engines"));
    for d in &lib.defs {
        println!(
            "  {}  {}  (source: {})",
            out::cyan(&d.id),
            d.name,
            d.repo.as_deref().unwrap_or("url")
        );
    }
}

fn detect_mod(path: &std::path::Path) -> hyper_core::Result<i32> {
    if !path.is_dir() {
        out::fail(&format!("not a directory: {}", path.display()));
        return Ok(1);
    }
    let settings = Settings::load()?;
    let lib = EngineLibrary::load()?;
    let det = detect::detect(path, &lib, &settings);
    match &det.engine_id {
        Some(id) => {
            println!(
                "{} {} ({})",
                out::cyan(id),
                out::dim(&path.display().to_string()),
                det.confidence.as_str()
            );
            for c in &det.candidates {
                println!("  candidate {:<12} score {}", c.engine_id, c.score);
            }
            for ev in &det.evidence {
                out::note(ev);
            }
            Ok(0)
        }
        None => {
            out::warn("no engine detected for this folder");
            Ok(1)
        }
    }
}

fn dir_size(p: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                total += dir_size(&e.path());
            } else if let Ok(m) = e.metadata() {
                total += m.len();
            }
        }
    }
    total
}

fn human_size(b: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if b >= (1024 * 1024 * 1024) as u64 {
        format!("{:.1} GiB", b as f64 / (1024.0 * 1024.0 * 1024.0))
    } else {
        format!("{:.1} MiB", b as f64 / MB)
    }
}
