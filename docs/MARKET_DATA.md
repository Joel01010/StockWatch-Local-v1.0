# Market Data Architecture

StockWatch Local keeps market data behind a replaceable provider contract. Portfolio accounting never depends on a provider-specific payload.

## Pipeline

`holdings + watchlist + alert targets -> normalized instruments -> provider adapter -> normalized Quote -> SQLite cache/history -> runtime snapshot -> tray/dashboard`

Every quote retains the exchange, local trading symbol, numeric price, optional daily percentage change, observation timestamp, actual source, and freshness classification.

## Provider policy

The default provider setting is `auto`.

### 1. Upstox Analytics Token — preferred official source

When the environment variable `STOCKWATCH_UPSTOX_TOKEN` is present, automatic mode creates an `upstox_analytics` provider first.

StockWatch:

1. resolves each NSE/BSE equity symbol through Upstox Instrument Search;
2. caches the resulting `instrument_key` in memory for the process lifetime;
3. requests exchange-backed snapshots from the Upstox Full Market Quote API;
4. records the actual trade/feed timestamp returned by the API;
5. labels REST snapshots `fresh`, not `live`, because StockWatch is polling rather than maintaining a streaming WebSocket.

The Analytics Token is read-only. StockWatch never stores its value in SQLite, settings JSON, logs, support bundles, or the frontend. Only the boolean "token configured" state is exposed to the UI.

The token is read from the process environment at startup:

```text
STOCKWATCH_UPSTOX_TOKEN
```

Restart StockWatch after adding or replacing the token.

For unusual symbols, `instrument_provider_symbols` can still store an explicit Upstox `instrument_key` override for provider id `upstox_analytics`.

### 2. Yahoo delayed — fallback

`yahoo_delayed` remains a best-effort public fallback. It maps `NSE:SYMBOL` to `SYMBOL.NS` and `BSE:SYMBOL` to `SYMBOL.BO`.

Yahoo quotes are always emitted as `delayed`. They are never promoted to `fresh` or `live`.

When automatic mode has an Upstox token, StockWatch asks Upstox first and uses Yahoo only for instruments that Upstox did not return or when the primary request fails. Each resulting quote preserves its real source, so a mixed refresh can contain both `upstox_analytics` and `yahoo_delayed` rows.

If no Upstox token is configured, automatic mode uses Yahoo delayed without failing application startup.

## Freshness

Default local thresholds:

- `fresh` for an official Upstox REST snapshot while younger than the stale threshold;
- `delayed` for Yahoo while younger than the stale threshold;
- `stale` after 5 minutes by default;
- `offline` after 6 hours by default.

Cache aging can only downgrade freshness. It never upgrades delayed data.

## No fabricated portfolio values

The Tauri dashboard no longer substitutes demo holdings when the database is empty or unavailable.

If there are no imported holdings, the UI reports an empty portfolio. If a backend read fails, the UI reports the error and unavailable values.

The overview performance chart reads only verified local `performance_snapshots`. If there is no history, it shows an explicit empty state rather than a generated demonstration curve.

## Scheduling

The scheduler estimates normal NSE weekday hours as 09:15–15:30 IST. This remains a local clock estimate rather than an authoritative exchange-holiday calendar. Closed-market polling is disabled by default.

Failures use bounded exponential backoff and do not delete the last known good quote. Partial responses update successful instruments while prior cached values for failures age naturally to stale/offline.

## Privacy and resource behavior

No AI is involved. Providers are contacted only for configured holdings, watchlist items, benchmarks, and enabled alert targets. Quotes and provider health stay local in SQLite.

The background worker reuses HTTP clients. The dashboard remains on-demand. Upstox credentials are never committed to the repository and must not be placed in CSV imports, provider-symbol overrides, or diagnostic files.
