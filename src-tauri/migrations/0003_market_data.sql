PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS market_provider_state (
    provider_id TEXT PRIMARY KEY,
    consecutive_failures INTEGER NOT NULL DEFAULT 0 CHECK(consecutive_failures >= 0),
    last_success_at TEXT,
    last_error TEXT,
    next_retry_at TEXT
);

CREATE TABLE IF NOT EXISTS instrument_provider_symbols (
    instrument_id INTEGER NOT NULL REFERENCES instruments(id) ON DELETE CASCADE,
    provider_id TEXT NOT NULL,
    provider_symbol TEXT NOT NULL,
    PRIMARY KEY(instrument_id, provider_id),
    UNIQUE(provider_id, provider_symbol)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_quote_history_dedupe
ON quote_history(instrument_id, source, observed_at);

INSERT OR REPLACE INTO schema_meta(key, value) VALUES ('schema_version', '3');
