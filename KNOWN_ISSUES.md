# Known issues — v1.0 RC2

## RC2 requires fresh qualification

RC2 changes the market-data provider path, dashboard fallback behavior, overview history wiring, diagnostics and release packaging. RC1 qualification results must not be used to certify this source state.

Run `START_HERE.cmd` on the target Windows machine before release. Installed-app qualification remains a separate explicit gate.

## Upstox credential is external to app settings

Official Upstox snapshots require `STOCKWATCH_UPSTOX_TOKEN` to exist in the process environment before StockWatch starts.

The token is intentionally not stored in SQLite or settings JSON and is never returned to the frontend. A future release may add Windows Credential Manager integration for easier setup.

If the token is absent, Automatic provider mode uses Yahoo delayed rather than failing startup.

## Upstox mode is REST polling, not WebSocket streaming

RC2 uses Instrument Search plus Full Market Quote REST snapshots. These are exchange-backed snapshots but are classified `fresh`, not `live`.

A future WebSocket adapter may use `live` freshness after its own native qualification.

## Public fallback is delayed/best-effort

Yahoo remains a fallback and is deliberately labeled `delayed`. It can have symbol gaps or provider-side outages. A fallback quote never inherits the Upstox source label.

## Market-hours calendar is approximate

Normal NSE weekday hours are modeled locally. Exchange holidays and special sessions are not yet driven by an authoritative holiday calendar.

## Last-window exit guard still needs shutdown/logoff regression coverage

StockWatch prevents a code-less Tauri `ExitRequested` event after the last WebView closes so the tray backend stays alive. Explicit tray Exit remains allowed. Windows shutdown/logoff behavior must remain part of native qualification.

## Visual Windows toast verification

The notification API can be tested automatically, but actual visual toast rendering/branding must not be claimed unless it is observed or captured in the installed-app context.

## Sleep/resume test is opt-in

`DIAG_SLEEP_RESUME.ps1` remains excluded from automatic diagnostics because it can suspend the machine. It requires explicit `-AllowSuspend` authorization.
