# Third-Party Notices

Hyper Engine is licensed under the Apache License 2.0 (see `LICENSE`).

This document lists the direct dependencies of Hyper Engine, the license each
declares in its own metadata, and where it is used. Transitive dependencies are
covered by `Cargo.lock`; run `cargo metadata` / `cargo license` for a complete,
machine-checked list. Where a crate is dual-licensed (`MIT OR Apache-2.0`) the
corresponding text is available in the crate's metadata or at
<https://choosealicense.com/licenses/> and <https://www.apache.org/licenses/>.

## Rust crates (direct)

| Crate | Declared license | Used by |
|---|---|---|
| anyhow | MIT OR Apache-2.0 | all crates |
| anstyle | MIT OR Apache-2.0 | hfe-cli |
| clap | MIT OR Apache-2.0 | hfe-cli |
| dirs | MIT OR Apache-2.0 | hyper-core |
| hex | MIT OR Apache-2.0 | hyper-core |
| hyper-core | Apache-2.0 | hfe-cli, hyper-launcher |
| hyper-pkg | Apache-2.0 | hfe-cli |
| keyring | MIT OR Apache-2.0 | hyper-core |
| libc | MIT OR Apache-2.0 | hyper-core |
| open | MIT OR Apache-2.0 | hyper-launcher |
| regex | MIT OR Apache-2.0 | hyper-core |
| reqwest | MIT OR Apache-2.0 | hyper-core |
| rpassword | Apache-2.0 OR MIT | hfe-cli |
| serde | MIT OR Apache-2.0 | all crates |
| serde_json | MIT OR Apache-2.0 | all crates |
| sha2 | MIT OR Apache-2.0 | hyper-core |
| tauri | MIT OR Apache-2.0 | hyper-launcher |
| tauri-build | MIT OR Apache-2.0 | hyper-launcher (build) |
| thiserror | MIT OR Apache-2.0 | hyper-core, hyper-pkg |
| toml | MIT OR Apache-2.0 | hyper-core |
| url | MIT OR Apache-2.0 | hyper-core |
| walkdir | MIT OR Unlicense | hyper-core |
| windows-sys | MIT OR Apache-2.0 | hyper-core |
| zip | MIT | hyper-core |

Note: `rpassword` is a transitive user of the system `security` CLI on macOS;
passwords are only ever passed to the operating system keyring.

## Sandboxes and runtimes

- **WebView2 (Windows)** — Microsoft WebView2 runtime. Its redistributable
  terms apply when you use the Tauri desktop launcher on Windows.
- **webkit2gtk (Linux)** — LGPL-2.1+ system library, linked dynamically by
  Tauri; it ships with your distribution, not with Hyper Engine.
- **SaraHUD** is a separate project by its own authors and is downloaded by
  Hyper Engine only with explicit consent, under SaraHUD's own license.

## Branding

"Friday Night Funkin'", "Psych Engine", "Codename Engine", "FPS+"/"Psych
Plus", "Sara", and "SaraHUD" are the trademarks or property of their
respective owners. Hyper Engine uses none of their logos or game assets and
is not affiliated with them.