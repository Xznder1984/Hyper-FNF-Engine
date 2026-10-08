# hyper-launcher

Tauri 2 desktop launcher for Hyper Engine. Detects which FNF engine a mod was
built for, fetches the exact engine (from official sources only), installs it
side-by-side in the shared store, optionally installs SaraHUD first in mod
order, then stages and launches the mod.

This crate shares its data directories with the `hfe` CLI (settings, engine
store, cache, overrides), so CLI and GUI see the same state.

## Layout

- `src/` — Rust backend. Thin `#[tauri::command]` wrappers around `hyper-core`;
  the real flow lives in `hyper-core`.
- `frontend/` — checked-in static UI (HTML/CSS/JS). No build step, WCAG 2.2 AA
  oriented: keyboard operable, focus-visible rings, 44px targets, high-contrast
  themes for light and dark, `prefers-reduced-motion` respected.
- `haxe/` — canonical UI source of record (Haxe, JS target). `app.js` in
  `frontend/` is the hand-maintained JS fallback and should match the bridge
  types declared in `haxe/Main.hx`. To regenerate:
  `cargo run -p hyper-launcher` is not needed; just `haxe build.hxml` in `haxe/`.
  Haxe is **not** required to build or run the launcher.
- `capabilities/default.json` — Tauri 2 ACL for the main window.
- `icons/` — generated artwork (see `tools/gen-icons.js`; original, no FNF assets).
- `tauri.conf.json` — Tauri 2 configuration.

## Build

```sh
cargo build -p hyper-launcher
```

Bundling per platform (installers/app bundles):

```sh
cargo install tauri-cli --locked
cargo tauri build
```

> Note: on Windows the WebView2 runtime is used (preinstalled on Win10/11).
> A full `tauri build` (bundling) additionally needs system dependencies for
> the target platform; CI uses runners that provide them.

## Commands (bridge)

`detect_mod`, `engines`, `settings_get`, `remember_engine`, `forget_engine`,
`set_game_assets`, `sarahud_enabled`, `launch_mod`, `open_external`.
Progress is streamed to the UI over the `hyper://progress` event.