# Phase 3 Dashboard & Mini Mode

## Goals
Phase 3 turns the Phase 2 tray-first runtime into a usable desktop portfolio surface without moving portfolio calculations into the WebView. The UI remains disposable: Tauri creates the main dashboard only on demand and the default close policy destroys the WebView.

## Routes
- `#/` — overview
- `#/holdings` — complete holdings table
- `#/holding/<SYMBOL>` — holding detail
- `#/watchlist` — Phase 4 placeholder
- `#/alerts` — Phase 5 placeholder
- `#/analytics` — Phase 6 placeholder
- `#/settings` — Phase 5 settings placeholder
- `#/mini` — compact quick-view surface

Routing is intentionally dependency-free to reduce frontend weight.

## Data boundary
The Svelte layer calls `dashboard_snapshot`. Rust opens the local SQLite database under the application data directory, derives the current portfolio from the transaction ledger, merges any in-memory runtime quotes and returns a presentation DTO. If the ledger has no holdings or Tauri IPC is unavailable, the frontend falls back to the supplied screenshot values and labels them `OFFLINE DEMO`.

The frontend does not calculate canonical cost basis or realized P&L.

## Chart policy
Phase 3 supplies a dependency-free SVG chart surface. It deliberately uses a marked demonstration series because historical portfolio snapshots are not implemented until Phase 6. No chart package is loaded into the background process or dashboard bundle.

## Mini mode
The native tray exposes `Quick View`. Tauri creates a separate `mini` WebView only when requested. It is configured as a compact, undecorated, always-on-top, taskbar-skipping window. Actual positioning, focus-loss dismissal and Windows behavior are intentionally deferred to Phase 8 qualification.

## Resource rules preserved
- no dashboard window at startup;
- no Python or Node background service;
- no charting daemon;
- no AI components;
- no network provider in Phase 3;
- closing the default dashboard releases its WebView rather than keeping a hidden UI resident.
