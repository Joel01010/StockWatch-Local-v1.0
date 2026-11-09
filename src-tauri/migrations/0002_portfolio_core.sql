PRAGMA foreign_keys = ON;

CREATE INDEX IF NOT EXISTS idx_transactions_instrument_time
    ON transactions(instrument_id, executed_at, id);

CREATE INDEX IF NOT EXISTS idx_transactions_external_ref
    ON transactions(external_ref)
    WHERE external_ref IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_instruments_symbol
    ON instruments(symbol);

CREATE TABLE IF NOT EXISTS import_batches (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('transactions','holdings_snapshot')),
    source_name TEXT,
    imported_at TEXT NOT NULL,
    row_count INTEGER NOT NULL DEFAULT 0 CHECK(row_count >= 0),
    inserted_count INTEGER NOT NULL DEFAULT 0 CHECK(inserted_count >= 0),
    skipped_count INTEGER NOT NULL DEFAULT 0 CHECK(skipped_count >= 0)
);

INSERT OR REPLACE INTO schema_meta(key, value) VALUES ('schema_version', '2');
