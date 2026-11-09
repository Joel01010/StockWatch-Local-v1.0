PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS cash_flows (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('deposit','withdrawal','dividend','interest')),
    amount REAL NOT NULL CHECK(amount > 0),
    occurred_at TEXT NOT NULL,
    note TEXT,
    external_ref TEXT UNIQUE
);
CREATE INDEX IF NOT EXISTS idx_cash_flows_time ON cash_flows(occurred_at, id);

CREATE TABLE IF NOT EXISTS performance_snapshots (
    id INTEGER PRIMARY KEY,
    bucket_unix_ms INTEGER NOT NULL UNIQUE,
    observed_at TEXT NOT NULL,
    portfolio_value REAL NOT NULL CHECK(portfolio_value >= 0),
    open_cost_basis REAL NOT NULL CHECK(open_cost_basis >= 0),
    realized_gross_pnl REAL NOT NULL,
    fees_paid REAL NOT NULL CHECK(fees_paid >= 0),
    investment_income REAL NOT NULL DEFAULT 0,
    net_external_flow REAL NOT NULL DEFAULT 0,
    source TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_performance_snapshots_time ON performance_snapshots(observed_at);

CREATE TABLE IF NOT EXISTS performance_holding_snapshots (
    snapshot_id INTEGER NOT NULL REFERENCES performance_snapshots(id) ON DELETE CASCADE,
    instrument_id INTEGER NOT NULL REFERENCES instruments(id) ON DELETE CASCADE,
    quantity REAL NOT NULL CHECK(quantity >= 0),
    price REAL NOT NULL CHECK(price >= 0),
    current_value REAL NOT NULL CHECK(current_value >= 0),
    open_cost_basis REAL NOT NULL CHECK(open_cost_basis >= 0),
    PRIMARY KEY(snapshot_id, instrument_id)
);

CREATE TABLE IF NOT EXISTS analytics_preferences (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    benchmark_exchange TEXT,
    benchmark_symbol TEXT,
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO analytics_preferences(id, benchmark_exchange, benchmark_symbol, updated_at)
VALUES(1, NULL, NULL, '1970-01-01T00:00:00Z');

INSERT OR REPLACE INTO schema_meta(key, value) VALUES ('schema_version', '5');
