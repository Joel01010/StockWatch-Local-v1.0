# Changelog

## 1.0.0-rc.2 — Market-data truthfulness and official-source update

- Added an Upstox Analytics Token market-data adapter using Instrument Search plus Full Market Quote snapshots.
- Added automatic provider selection: Upstox official snapshots first when `STOCKWATCH_UPSTOX_TOKEN` is configured, Yahoo delayed as explicit fallback.
- Preserved per-quote source provenance so fallback quotes are never mislabeled as Upstox.
- Classified Upstox REST polling as `fresh` rather than `live`; streaming/WebSocket is not claimed.
- Removed automatic demo-portfolio substitution when the local database is empty or unavailable.
- Replaced the fabricated overview performance curve with persisted local analytics history.
- Removed stale Phase 6 product labels from the application shell.
- Added safe UI reporting for whether an Upstox token is configured without exposing the credential value.
- Added optional Upstox market-data diagnostics that never log the token.
- Hardened source checks against reintroducing fabricated dashboard values or Yahoo-only defaults.
- Fixed Windows packaging after public-repo sanitization and made package names derive from the Tauri version.
- RC2 requires a fresh Windows/Rust/frontend qualification run; RC1 PASS evidence is historical and does not certify RC2.

## 1.0.0-rc.1 — Windows Release Candidate

- Added native Tauri autostart plugin integration and Settings synchronization.
- Added tray-first process lifetime guard so destroying the last dashboard WebView does not terminate the background monitor.
- Added inert qualification hooks for dashboard/Quick View creation, explicit exit, native autostart and installed notification smoke tests.
- Added `QUALIFY_WINDOWS.ps1` for automated Windows build/native lifecycle/resource qualification.
- Added `QUALIFY_INSTALLED_WINDOWS.ps1` for installed-build notification qualification.
- Added `PACKAGE_WINDOWS_RC.ps1` for Windows binary/installer RC packaging.
- Added Phase 8 release/source invariants and release reference tests.
- Synchronized package/Cargo/Tauri version to `1.0.0-rc.1`.
- Limited the release bundle target to NSIS to avoid unnecessary MSI/VBScript build requirements.
- Preserved the no-WebView-at-startup architecture.
- Updated Phase 7 version regression check so later release versions do not falsely fail an earlier-phase invariant.

## 0.7.0-p7 — Reliability & Optimization

See the previous Phase 7 checkpoint for the full reliability/hardening change list.

## RC1 automated-test-pack hardening — 2026-11-08

- Added `START_HERE.cmd` one-command safe Windows qualification entry point.
- Added automatic focused diagnostics and one-file support-bundle output contract.
- Added literal instructions suitable for a very small local model.
- Added isolated AppData for runtime diagnostics and qualification.
- Added qualification-mode suppression of normal autostart synchronization.
- Added autostart pre-state restore after tests.
- Added safe installed-app test refusal when an existing StockWatch install is present.
- Added automatic cleanup of the test installation by default.
- Added opt-in-only sleep/resume diagnostic.
- Added harness structure/safety regression checks.
- Regression suite increased to 62 passing tests.
