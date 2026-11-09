# Known issues — v1.0 RC1

## Native Windows qualification is not executed in this runner

This environment is Linux, not Windows. Actual tray visibility/right-click behavior, native WebView lifecycle, Windows sign-out/shutdown behavior, notifications, startup registration across a real login, sleep/hibernate/resume, installer/uninstaller and real Windows resource measurements are therefore **not claimed as passed**. The project includes automated Windows qualification scripts to run those checks on the target environment.

## Rust compile gate unavailable here

`cargo`/`rustc` are not installed in this runner, and outbound package-host DNS is unavailable, so Rust compile/test/rustfmt/clippy could not be executed here. The Windows qualification script treats these as mandatory gates and requires Rust >=1.90, matching current Tauri/plugin MSRV requirements.

## Svelte/Vite production bundle unavailable here

`node_modules` is absent. `npm install` timed out because this runner cannot resolve the package registry, so the complete production Svelte/Vite bundle could not be executed. Dependency-free TypeScript checks pass.

## Last-window exit guard needs real shutdown/logoff validation

StockWatch intentionally prevents a code-less Tauri `ExitRequested` event after the last WebView closes so the tray backend stays alive. Explicit tray `Exit` uses `app.exit(0)` and remains allowed. A real Windows sign-out/shutdown must be checked to ensure this guard does not obstruct session termination.

## Public market feed is delayed/best-effort

The built-in public provider is deliberately labeled delayed. It is not an exchange-authorized real-time NSE/BSE feed and can have symbol gaps. Provider-symbol overrides are supported.

## Market-hours calendar is approximate

Normal NSE weekday hours are modeled, but exchange holidays and special sessions are not yet driven by an authoritative holiday calendar.

## Package lock not generated in this environment

Because npm registry access is unavailable here, this source checkpoint does not include a freshly generated `package-lock.json`. The Windows build step will resolve dependencies unless a lockfile is created first in a networked environment.

## Automated test-pack limitations

- The PowerShell harness has been source/structure checked here but **has not been executed on a Windows host in this environment**. `START_HERE.cmd` is the next real validation gate.
- Installed notification qualification can verify the Tauri/Windows notification API returns success, but visual toast rendering/branding is not claimed from Linux-side checks.
- `DIAG_SLEEP_RESUME.ps1` is intentionally excluded from automatic diagnostics because it can suspend the machine; it requires explicit `-AllowSuspend` authorization.
- Package installation/build still depends on npm/crates network access when dependencies are not already cached.

## Windows qualification completed
Safe and installed-app qualification were executed on Windows. Safe qualification: PASS. Installed-app qualification: PASS. Native toast visual rendering remains UNVERIFIED_AUTOMATICALLY; optional sleep/resume remains explicit opt-in.
