# StockWatch Local v1.0 RC2 test status

RC2 changes the market-data provider path and dashboard fallback/history behavior. The previous RC1 Windows qualification remains useful historical evidence, but it does **not** certify RC2.

| Gate | RC2 status |
|---|---|
| Source/provider implementation review | Implemented |
| Upstox adapter unit tests | Added; execution pending Windows/Rust qualification |
| Node/reference regression suite | **PENDING rerun** |
| Phase 0–8 structural/source checks | **PENDING rerun** |
| SQLite/schema checks | **PENDING rerun** |
| TypeScript core + Tauri bridge | **PENDING rerun** |
| Rust cargo test/fmt/clippy | **PENDING rerun** |
| Svelte/Vite production build | **PENDING rerun** |
| Yahoo fallback network diagnostic | **PENDING rerun** |
| Upstox authenticated market-data diagnostic | **PENDING token-configured run** |
| Windows Tauri/NSIS build | **PENDING rerun** |
| Native tray/dashboard/mini lifecycle | **PENDING rerun** |
| Installed-app qualification | **PENDING rerun** |
| Visual Windows toast verification | **PENDING** |

Use `START_HERE.cmd` for the safe RC2 qualification. If installed-app testing is explicitly desired, use `START_FULL_WINDOWS_TEST.cmd`.

A release must not be called final v1.0 until the RC2 gates are rerun and recorded.
