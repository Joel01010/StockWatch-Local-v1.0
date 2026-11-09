# Alerts and Settings — Phase 5

## Goal

Phase 5 adds a deterministic, local alert engine and persistent settings without adding an AI service or a second background process. Alert evaluation runs inside the existing Rust/Tauri process and uses the same normalized runtime quote state as the tray and dashboard.

## Alert conditions

Supported conditions:

- price above / below
- daily change above / below
- holding return above / below
- holding P&L above / below
- portfolio value above / below
- portfolio P&L above / below
- feed age above a threshold (`feed_stale`)

Symbol alerts may target a holding or a manually entered NSE/BSE symbol. Enabled alert-only instruments are included in the market-data target set so they can be monitored without first adding a holding.

## Repeat behavior

Three repeat modes are supported:

1. `once` — fires once, then disables itself.
2. `crossing` — requires an inactive-to-active threshold crossing. After firing it disarms until the metric moves past the configured reset margin (hysteresis).
3. `repeat_while_true` — may repeat while the condition remains true, but only after the configured cooldown.

Crossing rules deliberately do not fire on the first sample when the application starts already beyond the threshold. This prevents restart storms from cached state.

## Freshness safety

Price, percentage, holding P&L/return and portfolio-value/P&L alerts only evaluate against quotes classified as `live`, `fresh` or `delayed`. A quote classified `stale` or `offline` is not allowed to trigger a trading-value alert.

`feed_stale` is the exception: it intentionally evaluates quote age, including during provider failures. The background loop ages cached quotes and evaluates alert state even when network refresh fails or closed-market polling is disabled.

## Quiet hours and market hours

Quiet hours are a notification-delivery policy. A trigger occurring during quiet hours is recorded with `suppressed_quiet`; it is not replayed later. Rules may also be configured as normal-market-hours-only. The current market-hours check is the Phase 4 clock estimate and is not an authoritative exchange-holiday calendar.

## Persistence

Schema v4 adds:

- `alert_rule_meta` for repeat mode, market-hours-only policy, armed state, last metric value and timestamps.
- `alert_events` for local notification history and explicit delivery status.
- the existing `settings` table stores the normalized `app_settings` JSON object.

Runtime state is persisted after every evaluation, so restarting the app does not silently re-arm a crossing rule or reset cooldown history.

## Notification delivery

The Rust backend uses `tauri-plugin-notification`. Delivery status is recorded as one of:

- `sent`
- `suppressed_quiet`
- `suppressed_market` (reserved for an explicit delivery-stage market suppression policy)
- `notification_error`
- `recorded_only` (notifications globally disabled)

Actual Windows toast behavior, permission prompts, sounds and Action Center integration are intentionally deferred to the final Windows qualification phase.

## Settings

The Settings UI persists:

- launch-at-startup intent (native registration deferred to final Windows qualification)
- tray visibility/content and left-click behavior
- destroy-vs-hide WebView close policy
- provider and timeout settings
- open/closed market refresh cadence
- failure-backoff ceiling
- quote stale/offline thresholds
- quote history toggle
- startup refresh
- notification enable/sound/quiet hours
- default alert cooldown
- notification-history retention

Provider construction-time settings still require an application restart; settings that are read dynamically by the background worker are applied without a restart.

## Resource model

No alert polling thread is added. The alert engine is called from the existing market worker after cache aging and after refresh outcomes. SQLite remains the only persistent store. The dashboard is still disposable and is not required for alerts to run.
