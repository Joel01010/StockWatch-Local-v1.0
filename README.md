# StockWatch Local v1.0 RC1

A local-first, non-AI stock portfolio monitor designed to live primarily in the Windows system tray. The background runtime is Rust/Tauri; the Svelte/WebView dashboard is created only on demand.

## Start here for Windows testing

**Normal operator/model instruction: run `START_HERE.cmd`. Do not choose individual scripts.**

`START_HERE.cmd` is intentionally designed so a very small local model can run the test package without diagnosing the machine first. It:

1. performs deterministic preflight checks;
2. runs the complete safe Windows qualification when prerequisites permit it;
3. automatically runs every safe focused diagnostic whether the main gate passes or fails;
4. uses isolated AppData for runtime probes so the real StockWatch database/settings are not touched;
5. restores the user's previous autostart state after the autostart probe;
6. collects machine-readable summaries and relevant logs;
7. creates exactly one `SUPPORT_BUNDLE_*.zip`;
8. writes `RETURN_THIS_FILE.txt` telling the operator/model exactly which ZIP to return.

For literal low-capability-model instructions, see `scripts/AI_OPERATOR_INSTRUCTIONS.txt`.

**Do not run `START_FULL_WINDOWS_TEST.cmd` unless installed-app testing is explicitly wanted.** It can install software. It refuses to overwrite an existing StockWatch installation and uninstalls only the test copy it created by default.

## RC1 status

Phases 0–7 are implemented and the Phase 8 release/qualification harness is included. In this build environment, all dependency-free software checks pass, but **native Windows qualification has not been executed** because the runner is Linux and has no Windows desktop session. Rust/Cargo and the complete Svelte/Vite bundle also could not be executed here because the required toolchain/dependencies are unavailable and outbound package-host DNS is blocked.

Run `START_HERE.cmd` on the Windows machine to close the safe Windows gates. See `docs/AUTOMATED_TEST_HARNESS.md` and `docs/WINDOWS_QUALIFICATION.md`.

## Stack

- Rust + Tauri 2 background/runtime
- Svelte 5 + TypeScript UI
- SQLite via rusqlite
- Tokio scheduling
- Reqwest market-data adapter
- Native Tauri tray, notification and autostart plugins

There is no Python daemon, Node background service, Electron runtime, cloud backend, AI/LLM component, Redis or PostgreSQL.

## Implemented product areas

- Transaction-ledger portfolio core with weighted-average cost basis and realized/unrealized P&L.
- CSV/opening-holdings import model with exact invested-amount preservation.
- Native tray model and on-demand dashboard/Quick View lifecycle.
- Delayed market-data provider abstraction, quote provenance, freshness, cache/history and backoff.
- Custom deterministic alerts with crossing, hysteresis, cooldown, quiet hours and notification history.
- Configurable tray, refresh, provider, backup, retention and notification settings.
- Cash-flow-aware analytics, Modified Dietz return, allocation/contribution and benchmark support.
- Verified SQLite backups, staged restore, integrity checks, bounded logs/history and maintenance.
- Native autostart registration wired to the Settings toggle.
- Windows RC qualification scripts for compile/build, native lifecycle, autostart, installed notifications and resource measurements.
- Self-diagnosing test harness with support-bundle collection and alternate focused diagnostics.

## Software checks in this checkpoint

- 62/62 Node/reference/regression tests pass.
- Test-harness structure check passes with 23 required harness artifacts.
- Phase 0–8 structural/source checks pass.
- SQLite migrations and invariants through schema v6 pass.
- TypeScript core and Tauri bridge checks pass.
- 25,000-cycle simulated refresh soak passes with SQLite integrity intact.

## Safe automated Windows qualification

Run only:

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

This adds the NSIS install/installed notification API gate. It refuses to overwrite an existing StockWatch installation and cleans up its own test installation by default.

Do not call this build final `v1.0` until the Windows qualification report is green and the remaining native-only checks in `docs/WINDOWS_QUALIFICATION.md` are resolved.


## Privacy of example data

All bundled portfolio examples, demo holdings, test fixtures, and diagnostic symbols are synthetic or generic public-market examples. The repository does not intentionally include a user's brokerage account data, local database, support bundle, API credentials, or machine-specific qualification artifacts.
