PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS instruments (
    id INTEGER PRIMARY KEY,
    exchange TEXT NOT NULL,
    symbol TEXT NOT NULL,
    name TEXT,
    isin TEXT,
    UNIQUE(exchange, symbol)
);

CREATE TABLE IF NOT EXISTS transactions (
    id INTEGER PRIMARY KEY,
    instrument_id INTEGER NOT NULL REFERENCES instruments(id),
    side TEXT NOT NULL CHECK(side IN ('buy', 'sell')),
    quantity REAL NOT NULL CHECK(quantity > 0),
    price REAL NOT NULL CHECK(price >= 0),
    fees REAL NOT NULL DEFAULT 0 CHECK(fees >= 0),
    executed_at TEXT NOT NULL,
    external_ref TEXT,
    UNIQUE(external_ref)
);

CREATE TABLE IF NOT EXISTS quotes (
    instrument_id INTEGER PRIMARY KEY REFERENCES instruments(id),
    price REAL NOT NULL CHECK(price >= 0),
    change_pct REAL,
    observed_at TEXT NOT NULL,
    source TEXT NOT NULL,
    freshness TEXT NOT NULL CHECK(freshness IN ('live','fresh','delayed','stale','offline'))
);

CREATE TABLE IF NOT EXISTS quote_history (
    id INTEGER PRIMARY KEY,
    instrument_id INTEGER NOT NULL REFERENCES instruments(id),
    price REAL NOT NULL CHECK(price >= 0),
    observed_at TEXT NOT NULL,
    source TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_quote_history_instrument_time ON quote_history(instrument_id, observed_at);

CREATE TABLE IF NOT EXISTS portfolio_snapshots (
    id INTEGER PRIMARY KEY,
    observed_at TEXT NOT NULL,
    invested REAL NOT NULL,
    current_value REAL,
    day_pnl REAL,
    total_pnl REAL
);

CREATE TABLE IF NOT EXISTS alerts (
    id TEXT PRIMARY KEY,
    instrument_id INTEGER REFERENCES instruments(id),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)),
    condition_json TEXT NOT NULL,
    cooldown_seconds INTEGER NOT NULL DEFAULT 900 CHECK(cooldown_seconds >= 0),
    reset_margin REAL,
    last_triggered_at TEXT
);

CREATE TABLE IF NOT EXISTS watchlists (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS watchlist_items (
    watchlist_id INTEGER NOT NULL REFERENCES watchlists(id) ON DELETE CASCADE,
    instrument_id INTEGER NOT NULL REFERENCES instruments(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(watchlist_id, instrument_id)
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT OR REPLACE INTO schema_meta(key, value) VALUES ('schema_version', '1');
