# StockWatch Local v1.0 RC2 test status

RC2 changes the market-data provider path and dashboard fallback/history behavior. RC1 Windows qualification remains historical evidence and does **not** certify RC2.

## Automated source/CI gates

GitHub Actions run `42595865100` completed successfully against RC2 source commit `9021d8ed63addb6c90a0011f75b84bea1b7e646e`.

| Gate | RC2 status |
|---|---|
| Node/reference regression suite | **PASS — 65/65** |
| Phase 0–8 structural/source checks | **PASS** |
| Test harness structural checks | **PASS** |
| SQLite/schema checks | **PASS** |
| Phase 7 25,000-refresh soak simulation | **PASS** |
| TypeScript core + Tauri bridge | **PASS** |
| Svelte/Vite production build | **PASS** |
| Rust `cargo fmt --check` | **PASS** |
| Rust `cargo clippy --all-targets -- -D warnings` | **PASS** |
| Rust `cargo test` | **PASS — 38/38** |

## Provider/native gates still required

| Gate | RC2 status |
|---|---|
| Yahoo fallback network diagnostic on target Windows host | **PENDING rerun** |
| Upstox authenticated quote diagnostic with a real Analytics Token | **PENDING token-configured run** |
| Windows Tauri/NSIS production build | **PENDING rerun** |
| Native tray/dashboard/mini lifecycle | **PENDING rerun** |
| Autostart and resource qualification | **PENDING rerun** |
| Installed-app qualification | **PENDING rerun** |
| Visual Windows toast verification | **PENDING** |

The CI suite intentionally does not contain an Upstox credential. It validates compilation, provider invariants and parsing/tests without exposing a real token.

Use `START_HERE.cmd` for safe RC2 Windows qualification. Configure `STOCKWATCH_UPSTOX_TOKEN` before starting the app/test harness to exercise the official provider. If installed-app testing is explicitly desired, use `START_FULL_WINDOWS_TEST.cmd`.

Do not call this build final `v1.0` until the Windows-native RC2 gates above are rerun and recorded.
