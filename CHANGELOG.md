# Changelog

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
