# Hyper Engine

**Unofficial** Friday Night Funkin' engine manager and launcher.

Hyper Engine detects which FNF engine a mod targets (Psych Engine, Codename
Engine, FPS+/Psych Plus, or the official game), fetches that exact engine from
**official sources only**, and launches the mod against it — side by side
without touching unrelated installs.

[Website](https://xznder1984.github.io/Hyper-FNF-Engine/) ·
[Downloads](https://github.com/Xznder1984/Hyper-FNF-Engine/releases) ·
[Changelog](CHANGELOG.md) · [Security](SECURITY.md)

> Not affiliated with, endorsed by, or sponsored by the Friday Night Funkin'
> team, Newgrounds, or any engine team. Ships no game assets.

## Install

The `hfe` CLI is a single static binary. The one-liners below fetch the latest
release asset for your OS, verify its published SHA-256, and install it. The
optional Tauri GUI launcher is a separate download from the
[releases page](https://github.com/Xznder1984/Hyper-FNF-Engine/releases).

**Linux / macOS**

```sh
curl -fsSL https://raw.githubusercontent.com/Xznder1984/Hyper-FNF-Engine/main/install.sh | sh
```

Installs to `~/.local/bin/hfe`. Options: `--dir DIR`, `--version TAG`
(or `HFE_VERSION=1.0.4`), `--repo OWNER/NAME`, `--yes`; env `HFE_DIR`,
`HFE_REPO`, `GITHUB_TOKEN`.

**Windows (PowerShell 5.1+)**

```powershell
irm https://raw.githubusercontent.com/Xznder1984/Hyper-FNF-Engine/main/install.ps1 | iex
```

Installs to `%LOCALAPPDATA%\hyper-engine\bin\hfe.exe` and adds that folder to
your user `PATH` (restart the shell afterwards). Options: `-Version TAG`,
`-Dir DIR`, `-Repo OWNER/NAME`, `-NoModifyPath`, `-Yes`.

To uninstall, delete the installed `hfe` binary (and, if you like, the store at
`%APPDATA%\hyper-engine` on Windows or `~/.local/share/hyper-engine` on Unix).
There is no telemetry and no background service.

## Highlights

- **Detection that reads the mod**, not your words — `pack.json` /
  `modsList.txt` (Psych), `metadata.json` / `main.xml` (Codename),
  `meta.json` with a uid (FPS+), `manifest.json` (official).
- **Official sources only.** HTTPS enforced, checksums verified, zip-slip
  guarded, engines stored side-by-side by tag.
- **CLI + desktop launcher.** `hfe` (Rust) and `hyper-launcher` (Tauri 2)
  share one local store.
- **SaraHUD opt-in**, official release, first in mod order, author credited.
- **Auto-update with rollback**, keyring-stored GitHub token, no telemetry.

## Build

Requires Rust ≥ 1.87 (developed against 1.95.0).

```sh
cargo build --workspace        # hfe + hyper-launcher + libs
cargo test --workspace         # 25 unit tests across lib crates
cargo clippy --workspace --all-targets -- -D warnings   # clean
```

The desktop launcher uses Tauri 2 and needs the usual per-platform WebView
tools (WebView2 on Windows, webkit2gtk-4.1 on Linux, WKWebView on macOS).

## CLI

```sh
hfe init [PATH] [--dry-run] [--yes]            # scaffold a mod folder / remember engine
hfe run [PATH] [--engine id] [--version tag]   # detect, fetch, stage, launch
hfe add engine <id> [version]                  # download + install an engine
hfe add mod <PATH> --engine <id>               # stage a mod into an installed engine
hfe add sarahud <psych|fps-plus>               # opt-in SaraHUD (official source)
hfe add token set|clear|status                 # keyring-stored GitHub token
hfe list engines | list mods <PATH> | list all # installed engines / detection
hfe engines list | show <id> | tags <id>       # definitions + release tags
hfe doctor                                     # environment diagnostics
hfe update check|install|rollback|status       # verified self-update
hfe bench extract <zip> | start <id> | all <zip>
hfe version
```

Run `hfe --help` (or `hfe <cmd> --help`) for the full reference.

## Layout

| Path | Content |
|---|---|
| `crates/hyper-core` | detection, download, store, launch, SaraHUD, update, keyring |
| `crates/hyper-pkg` | package registry client interface + manifest format |
| `crates/hfe-cli` | `hfe` command line |
| `crates/launcher` | Tauri 2 desktop launcher (frontend + Haxe->JS source) |
| `engines/` | machine-readable engine definitions (reviewable TOML) |
| `install.sh` / `install.ps1` | curl/PowerShell installers (SHA-256 verified) |
| `site/` | static website (GitHub Pages) + structure/link checker |
| `arch/` | AUR packaging (`hyper-fnf-engine`, `hyper-fnf-engine-git`) |
| `.github/workflows/` | CI, release (sha256.txt), GitHub Pages, gitleaks |

## Status

Early development (`0.1.0`). Detection, downloads, store, launch, SaraHUD, the
CLI, and the launcher scaffold are implemented with a passing test suite; see
[PROGRESS.md](PROGRESS.md) for exactly what is verified and what remains.

## License

Apache-2.0. See [LICENSE](LICENSE) and [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES.md).