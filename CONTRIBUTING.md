# Contributing

Thanks for wanting to help make Hyper Engine better.

## Ground rules

- Hyper Engine is **unofficial** and must stay that way. No FNF logos, no game
  art, no asserting affiliation with the FNF team, Newgrounds, or any engine
  team in contributed material.
- All contributions are Apache-2.0; submission is under the terms of section 5
  of the Apache License.
- No synthetic benchmark numbers. Numbers in docs and the README must be real
  measurements with their context (machine, command, timestamp).
- Keep downloads honest: official sources only, HTTPS, checksums.

## Getting started

```sh
git clone https://github.com/Xznder1984/Hyper-FNF-Engine.git
cd Hyper-FNF-Engine
cargo build --workspace
cargo test --workspace
```

On Windows, use an MSVC or GNU host that can build WebView2/Tauri. (The
workspace compiles clean on MSYS2 GNU.)

## What to work on

Check `PROGRESS.md` — items still unchecked are open. Good first areas:

- Engine definitions in `engines/*.toml` (documented, reviewable, tested).
- CLI UX (`crates/hfe-cli`), detection scoring (`crates/hyper-core`).
- Website content (`site/`) — run `node site/checks/sitecheck.mjs` before
  submitting.

## Pull request checklist

- [ ] `cargo build --workspace` is clean (no warnings).
- [ ] `cargo test --workspace` passes.
- [ ] New behavior has a test; fixes come with a failing test first where practical.
- [ ] No secrets committed (run gitleaks if available).
- [ ] `CHANGELOG.md` updated under "Unreleased".
- [ ] Docs (website or README) updated if user-facing behavior changed.

## Code style

- `rustfmt` default; clippy-clean.
- No opinionated comments; code should explain itself.
- Errors flow through `anyhow`/`thiserror` with user-friendly messages.

## Conduct

Be kind. See `CODE_OF_CONDUCT.md`. Harassment in any form is not tolerated and
will get you removed from the project.