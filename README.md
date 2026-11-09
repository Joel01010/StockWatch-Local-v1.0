# StockWatch Local v1.0 RC2

A local-first, non-AI Windows stock portfolio monitor designed to live primarily in the system tray. The background runtime is Rust/Tauri; the Svelte/WebView dashboard is created only on demand.

## Market data

The default provider mode is **Automatic**:

1. **Upstox Analytics Token** — preferred official source when `STOCKWATCH_UPSTOX_TOKEN` is available.
2. **Yahoo delayed** — best-effort fallback for missing/failed symbols or when no Upstox token is configured.

StockWatch resolves ordinary NSE/BSE equity symbols through Upstox Instrument Search, requests Full Market Quote snapshots, preserves the returned source/timestamp, and marks REST snapshots `fresh` rather than pretending they are a continuous streaming feed.

The Upstox token is read only from the process environment. Its value is not stored in SQLite, settings JSON, logs, support bundles, or the frontend.

See `docs/MARKET_DATA.md` for provider details.

## No fabricated portfolio data

The desktop app does **not** substitute demo holdings when the local portfolio is empty or when a backend call fails.

- No imported holdings → empty portfolio state.
- No usable market quote → unavailable/stale value with source/freshness shown.
- No verified performance snapshots → empty performance chart.

Bundled demo/test fixtures are synthetic and are not used as the user's portfolio at runtime.

## Start here for Windows testing

**Normal operator/model instruction: run `START_HERE.cmd`. Do not choose individual scripts.**

The safe runner:

1. performs deterministic preflight checks;
2. runs the complete safe Windows qualification when prerequisites permit it;
3. automatically runs every safe focused diagnostic whether the main gate passes or fails;
4. uses isolated AppData for runtime probes so the real StockWatch database/settings are not touched;
5. restores the user's previous autostart state after the autostart probe;
6. collects machine-readable summaries and relevant logs;
7. creates exactly one `SUPPORT_BUNDLE_*.zip`;
8. writes `RETURN_THIS_FILE.txt` identifying that bundle.

For literal low-capability-model instructions, see `scripts/AI_OPERATOR_INSTRUCTIONS.txt`.

**Do not run `START_FULL_WINDOWS_TEST.cmd` unless installed-app testing is explicitly wanted.** It can install software. It refuses to overwrite an existing StockWatch installation and uninstalls only the test copy it created by default.

## RC2 status

RC2 changes the market-data provider path and dashboard truthfulness behavior, so the earlier RC1 PASS evidence is historical only.

Run `START_HERE.cmd` on Windows to requalify RC2. See `TEST_RESULTS.md`, `docs/AUTOMATED_TEST_HARNESS.md`, and `docs/WINDOWS_QUALIFICATION.md`.

## Stack

- Rust + Tauri 2 background/runtime
- Svelte 5 + TypeScript UI
- SQLite via rusqlite
- Tokio scheduling
- Reqwest market-data adapters
- Native Tauri tray, notification and autostart plugins

There is no Python daemon, Node background service, Electron runtime, cloud backend, AI/LLM component, Redis or PostgreSQL.

## Implemented product areas

- Transaction-ledger portfolio core with weighted-average cost basis and realized/unrealized P&L.
- CSV/opening-holdings import model with exact invested-amount preservation.
- Native tray model and on-demand dashboard/Quick View lifecycle.
- Upstox official REST market snapshots with automatic symbol resolution and Yahoo delayed fallback.
- Quote provenance, freshness, cache/history and bounded failure backoff.
- Custom deterministic alerts with crossing, hysteresis, cooldown, quiet hours and notification history.
- Configurable tray, refresh, provider, backup, retention and notification settings.
- Cash-flow-aware analytics, Modified Dietz return, allocation/contribution and benchmark support.
- Verified SQLite backups, staged restore, integrity checks, bounded logs/history and maintenance.
- Native autostart registration wired to Settings.
- Windows qualification scripts for compile/build, native lifecycle, autostart, installed notifications and resource measurements.
- Self-diagnosing test harness with support-bundle collection and focused diagnostics.

## Safe automated Windows qualification

Run:

```text
START_HERE.cmd
```

When finished, return the ZIP named in:

```text
qualification-results\AUTO-*\RETURN_THIS_FILE.txt
```

## Full installed-app qualification

Only when explicitly requested:

```text
START_FULL_WINDOWS_TEST.cmd
```

This adds the NSIS install/installed-notification gate. It refuses to overwrite an existing StockWatch installation and cleans up its own test installation by default.

Do not call this build final `v1.0` until the RC2 qualification report is green.

## Privacy of example data

All bundled portfolio examples, demo holdings, test fixtures, and diagnostic symbols are synthetic or generic public-market examples. The repository does not intentionally include a user's brokerage account data, local database, support bundle, API credentials, or machine-specific qualification artifacts.
