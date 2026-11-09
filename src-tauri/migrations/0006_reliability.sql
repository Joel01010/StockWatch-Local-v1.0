PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS reliability_state (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT OR REPLACE INTO schema_meta(key, value) VALUES ('schema_version', '6');
