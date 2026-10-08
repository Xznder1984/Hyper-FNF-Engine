use clap::Subcommand;
use hyper_core::{bench::BenchReport, engine::EngineLibrary, launch, store};

use crate::out;

#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(subcommand)]
    cmd: BenchCmd,
}

#[derive(Subcommand, Debug)]
enum BenchCmd {
    /// Measure extraction throughput of an engine/mod archive
    Extract {
        zip: std::path::PathBuf,
        #[arg(long, default_value = ".")]
        dest: std::path::PathBuf,
    },
    /// Measure an installed engine's cold start (spawn -> running)
    Start {
        engine: String,
        #[arg(default_value = "latest")]
        version: String,
    },
    /// Run both benchmarks and print a measured-only report (JSON too)
    All { zip: std::path::PathBuf },
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
    let mut report = BenchReport::new();
    match args.cmd {
        BenchCmd::Extract { zip, dest } => {
            report.bench_extract(&zip, &dest)?;
        }
        BenchCmd::Start { engine, version } => {
            let lib = EngineLibrary::load()?;
            let def = lib
                .get(&engine)
                .ok_or_else(|| hyper_core::HyperError::NotFound(format!("engine '{engine}'")))?;
            let state = store::EngineState::load(def.id.as_str())?;
            let dir = match state.dir_for(&version) {
                Some(d) if d.is_dir() => d,
                _ => {
                    out::fail(&format!("{engine} {version} is not installed"));
                    return Ok(1);
                }
            };
            let exe = launch::find_executable(&dir).ok_or_else(|| {
                hyper_core::HyperError::NotFound(format!("no executable in {}", dir.display()))
            })?;
            report.bench_engine_start(&exe, &dir)?;
        }
        BenchCmd::All { zip } => {
            let lib = EngineLibrary::load()?;
            let s = store::installed_engines()?;
            let (id, dir) = s
                .iter()
                .flat_map(|st| st.versions.iter().map(|v| (st.id.clone(), v.dir.clone())))
                .next()
                .ok_or_else(|| {
                    hyper_core::HyperError::NotFound(
                        "install an engine first (hfe add engine psych)".into(),
                    )
                })?;
            let _ = lib;
            let tmp = std::env::temp_dir().join(format!("hfe-bench-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&tmp);
            report.bench_extract(&zip, &tmp)?;
            let _ = std::fs::remove_dir_all(&tmp);
            if let Some(exe) = launch::find_executable(&dir) {
                report.bench_engine_start(&exe, &dir)?;
            }
            let _ = id;
        }
    }
    println!("{}", out::bold("bench report (measured on this machine)"));
    println!("{}", serde_json::to_string_pretty(&report)?);
    out::note("in-game FPS is not included: engines expose their own debug overlays.");
    Ok(0)
}
