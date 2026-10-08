# Changelog

All notable changes to Hyper Engine are documented here. This project adheres
loosely to [Keep a Changelog](https://keepachangelog.com) and to
[Semantic Versioning](https://semver.org).

Entries under **Unreleased** describe the working tree; entries under a
version describe published releases.

## [Unreleased]

### Added
- `hyper-core` — FNF engine detection with per-engine scoring over metadata
  files and folder structure; decisive (High) classification when one engine
  owns a mod's metadata.
- `hyper-core` — engine store: side-by-side installs by tag, safe version
  directory names, download cache, SHA-256 verification.
- `hyper-core` — downloads: HTTPS-only policy, ETag caching, gzip/brotli,
  zip-slip guarded extraction (policy and traversal unit-tested).
- `hyper-core` — GitHub releases client with optional keyring-stored token and
  rate-limit handling; engine artifact picker.
- `hyper-core` — launch module: mod staging into a per-engine install,
  process launch, GPU/priority launch options.
- `hyper-core` — SaraHUD integration: opt-in, official source, first-in-order,
  credit author; guarded to Psych Engine and FPS+.
- `hyper-core` — update client with pinned manifests and automatic rollback.
- `hfe-cli` — `hfe` command line: `init`, `run`, `add`, `update`, `list`,
  `doctor`, `bench`, `pkg`, `engines`, plus common prompt flags.
- `install.sh` / `install.ps1` — curl/PowerShell installers that fetch the
  release archive plus its `.sha256`, verify the checksum, unpack, and print a
  PATH hint; flags for version/repo/target dir, env overrides, non-interactive
  safe (tty-gated prompt, `--yes`/`-y`), clear `error: download failed: ...`
  messages with a non-zero exit code. Validated with `bash -n`, the PowerShell
  parser, and a real bad-tag failure run.
- `hyper-pkg` — package registry client interface and manifest format.
- `hyper-launcher` — Tauri 2 desktop launcher sharing the CLI's store
  (detect, engines, settings, launching).
- `site/` — website (home, download, docs, sarahud, privacy, terms, cookies)
  with an automated link/structure checker.
- Engine definitions for Psych Engine, Codename Engine, FPS+ (Psych Plus), and
  the official game, as reviewable TOML with official sources.
- Legal set: `LICENSE`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`,
  `THIRD-PARTY-NOTICES.md`.

### Changed
- Workspace now includes `crates/launcher`; root `Cargo.toml` lists all four
  crates.
- README: added the Install section (curl one-liner, `irm | iex`, options and
  env vars, uninstall, no-telemetry note) and corrected the CLI reference and
  repository layout to match the shipped surface.
- CI: the validate job installs the Linux GTK/webkit2gtk system deps the Tauri
  crate needs (previously only the build job did), the workspace is
  rustfmt-formatted so `cargo fmt --check` passes, and GitHub Pages uses
  workflow-based deploys (enabled on the repository).

### Fixed
- Fresh installs had update checks silently disabled: `UpdateSettings` derived
  `Default` set `enabled: false` while the serde/default path used `true`;
  `Default` is now explicit and tested (`fresh_install_defaults_match_serde_defaults`).
- Self-update on Windows no longer fails with "os error 32": the live
  executable is renamed aside (`hfe.exe` -> `hfe.exe.old`) so the verified
  candidate can take its place; the old image is removed once unlocked, and a
  failed placement restores the previous binary instead of leaving a
  half-written one.
- `hfe update rollback` now picks the newest backup deterministically, restores
  the version record (`installed.json`) from the backup name, and fails with a
  clear message when the restore is blocked instead of reporting success.
- `hfe update status` prints whether update checks are enabled.
- `hfe init` works in an empty directory (previously failed to create the
  nested layout).
- Codename Engine definition points at `CodenameCrew/CodenameEngine` with the
  correct release asset patterns; FPS+ patterns updated to the official
  public repo's archives.
- `store::latest_dir` / `cmp_versions` order installed versions correctly
  (was string-ordering, so `1.10.0` lost to `1.9.0`).
- Launcher window icons are RGBA PNGs; Tauri's `generate_context!` rejects
  non-RGBA icons when building on Linux.
- The keyring roundtrip test skips (with a message) instead of panicking when
  the platform has no Secret Service, e.g. on CI runners; it still asserts
  fully wherever a real OS keyring exists.
- Detection avoided a race in parallel tests over a shared temp directory.
- `mod_folder_name` fallback now hashes the path when the folder has no
  final component.
- Engine definitions loaded `[detect.signatures]` / `[detect.metadata_scores]`
  tables (previously ignored).
- Embedded engine `id`s now match their TOML ids (`fps-plus`,
  `funkin-official`) so they are no longer dropped from lookup.

### Security
- HTTP downloads refused; update manifests pinned; no secrets in files.

## Notes
- Version `0.1.0` is the development milestone; first public release is still
  to be tagged.