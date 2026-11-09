# Phase 4 Market Data Architecture

StockWatch Local keeps market data behind a replaceable provider contract. Portfolio accounting never depends on a provider-specific payload.

## Pipeline

`holdings + watchlist -> normalized targets -> provider adapter -> normalized Quote -> SQLite cache/history -> runtime snapshot -> tray/dashboard`

Every quote carries an exchange, symbol, numeric price, optional daily percentage change, observation timestamp, source, and freshness classification.

## Built-in provider

Phase 4 includes `yahoo_delayed`, a best-effort public delayed-data adapter using the Yahoo Finance chart endpoint. It maps `NSE:SYMBOL` to `SYMBOL.NS` and `BSE:SYMBOL` to `SYMBOL.BO`. The adapter deliberately emits `delayed`, never `live`.

This provider is not treated as an exchange-certified trading feed. It is a convenient personal monitoring adapter and can be replaced by Zerodha or another authorized source without changing portfolio/storage/UI contracts. Per-instrument provider-symbol overrides are stored in `instrument_provider_symbols` for tickers that do not follow the default suffix convention.

## Freshness

Default local thresholds:

- provider baseline (`delayed` for the bundled adapter) while younger than 5 minutes;
- `stale` after 5 minutes;
- `offline` after 6 hours.

The original source and observation timestamp are retained. Cache aging can only downgrade freshness; it never upgrades delayed data to live.

## Scheduling

The scheduler estimates normal NSE weekday hours as 09:15–15:30 IST. This is a local clock estimate only, not an authoritative holiday/special-session calendar. Closed-market polling is disabled by default. Startup can perform one refresh so the latest available close is cached.

Failures use bounded exponential backoff and do not delete the last known good quote. Partial provider responses update successful instruments while leaving prior cached values for failed instruments, with age-based freshness degradation.

## Privacy and resource behavior

No AI is involved. The provider is contacted only for configured holdings/watchlist targets. Quotes and provider health are stored locally in SQLite. The background worker reuses one HTTP client and the dashboard remains on-demand.
