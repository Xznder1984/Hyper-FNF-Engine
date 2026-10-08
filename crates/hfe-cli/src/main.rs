mod cmd;
mod out;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "hfe",
    version,
    about = "Hyper Engine - run any FNF mod on the correct engine",
    long_about = "Hyper Engine is an unofficial Friday Night Funkin' mod launcher.\n\
It detects which engine a mod needs, fetches that exact engine version from its\n\
official source, and runs it with your mod. Hyper Engine is not affiliated with\n\
or endorsed by The Funkin' Crew Inc.; 'Friday Night Funkin'' and the game's art\n\
belong to their respective owners."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,

    /// Disable coloured output (NO_COLOR is also respected)
    #[arg(long, global = true)]
    no_color: bool,

    /// Be verbose (extra progress output)
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Prepare a mod folder/project for use with Hyper Engine
    Init(cmd::init::Args),
    /// Detect the engine for a mod, install it if needed, and launch it
    Run(cmd::run::Args),
    /// Add an engine, a mod, SaraHUD, or a GitHub token
    Add(cmd::add::Args),
    /// Check, install, or roll back launcher updates
    Update(cmd::update::Args),
    /// List installed engines / mods
    List(cmd::list::Args),
    /// Diagnose installation, store, and network issues
    Doctor,
    /// Measure launcher-side values only (extract throughput, cold start)
    Bench(cmd::bench::Args),
    /// Package sources; a full registry is coming soon
    Pkg(cmd::pkg::Args),
    /// Show engine definitions and versions
    Engines(cmd::engines::Args),
    /// Print version, install method, and store locations
    Version,
}

fn main() {
    let cli = Cli::parse();
    out::init(cli.no_color);

    let code = match cli.cmd {
        Cmd::Init(a) => cmd::init::run(a),
        Cmd::Run(a) => cmd::run::run(a),
        Cmd::Add(a) => cmd::add::run(a),
        Cmd::Update(a) => cmd::update::run(a),
        Cmd::List(a) => cmd::list::run(a),
        Cmd::Doctor => cmd::doctor::run(),
        Cmd::Bench(a) => cmd::bench::run(a),
        Cmd::Pkg(a) => cmd::pkg::run(a),
        Cmd::Engines(a) => cmd::engines::run(a),
        Cmd::Version => cmd::misc::version(),
    };
    std::process::exit(code);
}