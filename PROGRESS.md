# Hyper Engine — Progress Checklist

Live status file. When resuming, read this first, then continue from the first
incomplete item. Verified items are marked `[x]` only after a test actually ran.

## Build checklist

- [x] Git repo initialised (`main` branch), local identity set
- [x] Cargo workspace: hyper-core / hfe-cli / hyper-pkg / launcher compile clean
- [x] Engine definitions (psych, codename, fps-plus, funkin-official) + schema
- [x] Synthetic test mods for every engine (no copyrighted content)
- [x] Detection pipeline passes on all synthetic mods (DoD 2)
- [x] Downloads: HTTPS-only, cache, ETag, checksums, zip-slip guard tested
- [ ] Install store: side-by-side versions, GC with confirmation
      — unit-tested (`version_dir_safe`, `latest_dir`, `cmp_versions`) but only
      psych 1.0.4 was ever installed live; `store::prune_candidates` exists yet
      is not wired to any command, so there is no GC confirmation flow
- [x] Version resolution + engine launch (priority/GPU launcher-level)
- [x] `hfe init` in empty and non-empty folders, dry-run, non-interactive (DoD 1)
- [x] CLI: run/add/update/list/doctor/bench/pkg/engines; exit codes; NO_COLOR
- [x] Package registry client interface + "coming soon" pkg
- [x] SaraHUD: opt-in, first-in-mod-order, credit, author message draft (DoD 4)
- [x] Auto-update with rollback, fake older version test (DoD 5)
- [x] Benchmark (measured numbers only) — archive extract + engine cold start;
      per-mod profiles NOT implemented (bench covers extract + start only)
- [ ] Launcher GUI (Tauri 2 + Haxe->JS), WCAG 2.2 AA pass
      — runtime smoke passed (alive 24 s, 38 MB RSS, no stderr, stopped by
      harness); WCAG audit and an interactive GUI pass are still open
- [x] Website: home, download, docs, sarahud, privacy, terms, cookie; checks (DoD 8)
- [x] Legal: LICENSE, THIRD-PARTY-NOTICES, SECURITY, CONTRIBUTING, CoC
- [x] CI: win/mac/linux build matrix + checksums + gitleaks + pages deploy (DoD 7)
      — first push exposed and fixed three workflow issues (rustfmt drift,
      missing Linux GTK/webkit2gtk deps in the validate job, non-RGBA launcher
      icons rejected by Tauri codegen on Linux); after the fixes the full run
      is green: validate + gitleaks + all three OS release builds + Pages
- [ ] AUR PKGBUILD stable + -git; clean Arch container build (DoD 6)
      — files present and syntax-validated only; no Arch distro exists in this
      container, so the container build was NOT executed
- [x] E2E on this machine (DoD 3) — see evidence log; the local base game
      `D:\Albarr\Game\funkin-windows-64bit` was exercised with `--dry-run`
      (detection correctly reports low confidence — it has no mod metadata;
      `--engine funkin-official` resolves v0.8.7). That engine's full
      download+launch was not executed
- [x] Final honest report + commit + push (done in this session, see evidence)

## Test evidence log

- (verified 2026-10-09) `cargo build --workspace` clean, 0 warnings (incl. hyper-launcher)
- (verified 2026-10-09) `cargo test --workspace`: 27/27 pass (25 hyper-core + 2 hyper-pkg)
- (verified 2026-10-09) `cargo clippy --workspace --all-targets -- -D warnings`: clean
- (verified 2026-10-09) DoD 5 live on Windows, fake loopback manifest
  (`D:\Albarr\tmp\opencode\fake-update-test.ps1`, port 8799): `update check`
  0.1.0 -> 9.9.9; `update install --yes` downloaded, sha256-verified, created
  rollback backup `hyper-0.1.0.bin`, replaced the running exe via rename-aside,
  `installed.json` = 9.9.9, exit 0; `update rollback` restored 0.1.0 (record +
  binary), `hfe version` still ran, exit 0; full cleanup left no leftovers
- (verified 2026-10-09) DoD 4 live: `hfe add sarahud psych --yes` downloaded
  from the author's official releases into
  `...\psych\1.0.4\PsychEngine\mods\SaraHUD`, credit printed, and wrote
  `SaraHUD` first into `modsList.txt` (Psych 1.0.4's order file)
- (verified 2026-10-09) benchmark `hfe bench all PsychEngine-Windows64.zip`:
  extract 13.13 MiB/s (1220 files, 353.1 MiB), engine cold start 24 ms,
  engine survived first second; measured-only JSON report
- (verified 2026-10-09) launcher runtime smoke: `hyper-launcher.exe` alive 24 s,
  38 MB RSS, no stdout/stderr output, stopped by harness
- (verified 2026-10-09) `NO_COLOR=1 hfe doctor`: all checks pass, no ANSI codes
- (verified 2026-10-09) `hfe run <local base game> --engine funkin-official
  --dry-run` resolves v0.8.7; bare detection on that folder warns low confidence
- (verified 2026-10-09) full GitHub Actions run on the first release push is
  green: Secret scanning, Validate (fmt + site + clippy + tests), and release
  builds on ubuntu/windows/macos; Pages deploy green
- (verified 2026-10-09) site is live at https://xznder1984.github.io/Hyper-FNF-Engine/
  (fetched after the Pages deploy; home page renders with all nav links)
- (verified 2026-10-09) `cargo fmt --all -- --check` clean (workspace was
  formatted after CI enforced it); keyring roundtrip test skips cleanly when
  the platform has no Secret Service (CI runner) and still asserts on Windows
- (verified 2026-10-09) launcher window icons converted RGB -> RGBA (Tauri's
  `generate_context!` rejects non-RGBA PNGs on Linux; local Windows rebuild
  still clean)
- (verified 2026-10-09) installers: `install.sh` (`bash -n` + failure path
  prints `error: download failed: ...404`), `install.ps1` (PS 5.1 parser +
  same failure path); both exit 1 on a bad tag
- (verified 2026-10-08) Tauri 2 launcher compiles clean on the MSYS2 GNU host
  (webview2-com, tao, wry, tauri 2.12.1)
- (verified 2026-10-08) detection: psych/codename/fps-plus synthetic mods
  resolve to correct engine, High confidence; override wins
- (verified) embedded defs load (reads_embedded_psych); `[detect]` signatures +
  metadata_scores now parsed (was silently empty)
- (verified) zip-slip: extracts_clean_zip + rejects_parent_escape; download:
  blocks plain-HTTP remote + sha256 vector
- (verified) version resolve: exact+prefix, stable>prerelease, prerelease-only
  fallback
- (verified) store sanitize: version_dir_safe; launch: mod_folder_name + order parse
- (verified) SaraHUD: order prepend/remove idempotent, psych+fps-plus only
- (verified) hyper-pkg: explicit sources parse; registry "coming soon" fails honestly
- (verified) update: manifest artifact host matching
- (verified) `hfe init` empty-folder, non-empty-folder, run twice
- (verified) E2E launch: psych 1.0.4 real install (353.1 MiB, sha256) + real
  launch (engine pid observed, Psych log lines); codename/fps-plus/funkin
  resolved via `--dry-run` only
- (verified) site `sitecheck` PASS (home, download, docs, sarahud, privacy,
  terms, cookie; internal links + required headers)
- (verified) AUR PKGBUILD files parse (syntax-only; container build not run)

## Rules while building

- Never delete or overwrite user data without confirmation.
- Nothing copyrighted in the repo: no base-game assets, no engine builds.
- Only official sources; consent required for anything else (none planned).
- No fake numbers: benchmark/website only report what was actually measured.
