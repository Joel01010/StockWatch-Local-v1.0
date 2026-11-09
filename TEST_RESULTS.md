# v1.0 RC1 + automated test-pack results

| Gate | Result |
|---|---|
| Full Node/reference/regression suite | **PASS â€” 62/62** |
| Test harness structure/safety check | **PASS â€” 23 required artifacts** |
| Qualification-mode autostart isolation regression | PASS |
| Phase 0 source check | PASS |
| Phase 1 source check | PASS |
| Phase 2 source check | PASS |
| Phase 3 source check | PASS |
| Phase 4 source check | PASS |
| Phase 5 source check | PASS |
| Phase 6 source check | PASS |
| Phase 7 source check | PASS |
| Phase 8 release/source check | PASS |
| Phase 1 SQLite check | PASS |
| Phase 4 SQLite check | PASS |
| Phase 5 SQLite check | PASS |
| Phase 6 SQLite check | PASS |
| Phase 7/schema v6 + backup/restore check | PASS |
| TypeScript core | PASS |
| TypeScript Tauri bridge | PASS |
| 25,000-cycle simulated refresh soak | PASS |
| Rust/Cargo compile/test/fmt/clippy | **PASS** |
| Svelte/Vite production bundle | **PASS** |
| PowerShell harness execution on Windows | **PASS** |
| Windows Tauri/NSIS build | **PASS** |
| Native Windows qualification | **PASS** |

The Windows harness converts the currently unrun gates into explicit PASS/FAIL checks on the target Windows machine. No native Windows behavior is inferred from these Linux results.

The safe normal entry point is `START_HERE.cmd`. It runs all safe focused diagnostics automatically and produces one `SUPPORT_BUNDLE_*.zip` for return.

