# Phase 8 — Windows qualification

Phase 8 is the only stage allowed to claim native Windows behavior. Earlier phases deliberately used software/static/simulated checks only.

## Prerequisites

- 64-bit Windows 10/11 with Microsoft Edge WebView2 Runtime.
- Current Node.js/npm.
- Rust stable **1.90 or newer** (current Tauri 2.12 and current notification/autostart plugins require at least Rust 1.90).
- Microsoft Visual Studio C++ Build Tools / Desktop development with C++.
- PowerShell 7 recommended.

## Main automated gate

For a normal operator or local model, **do not choose scripts manually**. From the project root run:

```text
START_HERE.cmd
```

The master runner calls `QUALIFY_WINDOWS.ps1` itself, runs all safe fallback diagnostics, and creates one support ZIP named by `RETURN_THIS_FILE.txt`. Direct invocation of `QUALIFY_WINDOWS.ps1` is reserved for a human developer doing focused work.

The script performs:

1. Phase 0–8 regression/static/database checks.
2. Rust tests, rustfmt and clippy with warnings denied.
3. Svelte/Vite production build.
4. Tauri release + NSIS build.
5. Tray-only startup check: process alive and no visible WebView window.
6. Dashboard creation and automated `WM_CLOSE`; verifies the backend stays alive after the last WebView is destroyed.
7. Quick View creation/destruction and backend survival.
8. Explicit `app.exit(0)` check so the tray Exit path is not swallowed by the background lifetime guard.
9. Native autostart status/enable/disable/restore using the Tauri autostart plugin; the pre-test state is restored.
10. Real process working-set/private-memory, thread and handle snapshots.

Results are written under `qualification-results/<timestamp>/` and include a machine-readable `qualification-summary.json`.

## Installed-app notification gate

Only when installed-app testing is explicitly authorized, run:

```text
START_FULL_WINDOWS_TEST.cmd
```

The installed-app qualifier refuses to overwrite a pre-existing StockWatch installation. It silently installs the generated RC, exercises the native notification API, records the result, and uninstalls only the test copy it created unless `-KeepInstalled` is explicitly requested by a human developer. Visual toast rendering is still not inferred from API success alone.

## Manual Windows checks still required

Some user-interface behavior cannot be established by source inspection alone. On the target Windows machine verify:

- The tray icon is actually visible (it may initially be under the Windows overflow `^` menu).
- Right-click shows current configured holdings, portfolio metrics, refresh, Quick View, dashboard, settings and Exit.
- Left-click behavior follows the selected setting.
- `Exit` from the tray really terminates the process.
- A normal Windows sign-out/shutdown is not obstructed by the last-window exit guard.
- Notification sound/quiet-hours behavior matches settings.
- Enabling “Launch at Windows startup” survives a real sign-out/sign-in cycle and disabling it removes the registration.
- Sleep/hibernate/resume causes one recovery refresh and no duplicate worker.
- Network disconnect/reconnect preserves cached prices and marks freshness correctly.
- Opening/closing the dashboard repeatedly does not cause monotonically growing RAM, WebView2 child processes, handles or threads.
- Installer install/repair/uninstall works cleanly.

A release should not be called `v1.0` until these native checks are completed on Windows.
