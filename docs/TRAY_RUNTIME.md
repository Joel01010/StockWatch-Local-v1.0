# Phase 2 — Tray Runtime

## Purpose

StockWatch Local is tray-first. The always-on process is Rust; the Svelte/WebView dashboard is created only when requested.

## Runtime contract

- One Rust/Tauri application process owns portfolio state, refresh state, settings, and the native tray icon.
- No dashboard window is created at startup (`app.windows` is empty in `tauri.conf.json`).
- Right-click opens the native tray menu. `show_menu_on_left_click(false)` reserves left-click for a configurable action.
- Closing an on-demand dashboard destroys its WebView by default. The tray process remains alive.
- The tray menu is generated from a platform-neutral model and can be rebuilt after runtime state changes.
- `Refresh Now` uses an in-flight guard today; Phase 4 wires it to actual market providers.
- `OpenMiniMode` intentionally falls back to the dashboard until Phase 3 adds mini mode.

## Menu IDs

Stable IDs are deliberately used so platform-native events map to deterministic commands:

- `action:dashboard`
- `action:settings`
- `action:alerts`
- `action:refresh`
- `action:exit`
- `symbol:<SYMBOL>`

Display-only lines begin with `status:` and cannot be routed as actions.

## Current validation level

Phase 2 validates the menu model, command parsing, settings normalization, runtime state sharing, source structure, and configuration automatically. It **does not claim** that a tray icon was visually displayed or clicked on a real Windows desktop. Those checks are reserved for Phase 8.

## Windows event-safety detail

Tauri documents that `WebviewWindowBuilder::new` can deadlock on Windows when invoked directly from synchronous commands or event handlers. Tray actions therefore copy the `AppHandle` and request dashboard creation from a separate thread rather than constructing the WebView in the tray callback itself. This is a source-level safety measure; real Windows qualification remains deferred to Phase 8.
