PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS alert_rule_meta (
    alert_id TEXT PRIMARY KEY REFERENCES alerts(id) ON DELETE CASCADE,
    exchange TEXT,
    repeat_mode TEXT NOT NULL DEFAULT 'crossing' CHECK(repeat_mode IN ('once','crossing','repeat_while_true')),
    market_hours_only INTEGER NOT NULL DEFAULT 0 CHECK(market_hours_only IN (0,1)),
    armed INTEGER NOT NULL DEFAULT 1 CHECK(armed IN (0,1)),
    last_value REAL,
    created_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS alert_events (
    id INTEGER PRIMARY KEY,
    alert_id TEXT NOT NULL REFERENCES alerts(id) ON DELETE CASCADE,
    fired_at TEXT NOT NULL,
    metric_value REAL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    delivery_status TEXT NOT NULL CHECK(delivery_status IN ('sent','suppressed_quiet','suppressed_market','notification_error','recorded_only')),
    delivery_error TEXT
);
CREATE INDEX IF NOT EXISTS idx_alert_events_alert_time ON alert_events(alert_id, fired_at DESC);
CREATE INDEX IF NOT EXISTS idx_alert_events_time ON alert_events(fired_at DESC);

INSERT OR REPLACE INTO schema_meta(key, value) VALUES ('schema_version', '4');
