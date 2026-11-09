from pathlib import Path
import sqlite3

root = Path(__file__).resolve().parents[1]
conn = sqlite3.connect(':memory:')
conn.executescript((root/'src-tauri/migrations/0001_initial.sql').read_text())
conn.executescript((root/'src-tauri/migrations/0002_portfolio_core.sql').read_text())

version = conn.execute("SELECT value FROM schema_meta WHERE key='schema_version'").fetchone()[0]
assert version == '2', version

tables = {r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table'")}
expected = {'schema_meta','instruments','transactions','quotes','quote_history','portfolio_snapshots','alerts','watchlists','watchlist_items','settings','import_batches'}
missing = expected - tables
assert not missing, missing

indexes = {r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='index'")}
for idx in {'idx_transactions_instrument_time','idx_transactions_external_ref','idx_instruments_symbol'}:
    assert idx in indexes, idx

# SQLite-level idempotency and constraints.
conn.execute("INSERT INTO instruments(exchange,symbol,name) VALUES('NSE','TEST','Test')")
iid = conn.execute("SELECT id FROM instruments WHERE symbol='TEST'").fetchone()[0]
conn.execute("INSERT INTO transactions(instrument_id,side,quantity,price,fees,executed_at,external_ref) VALUES(?,?,?,?,?,?,?)", (iid,'buy',1,100,0,'2026-01-01T10:00:00Z','dup'))
conn.execute("INSERT INTO transactions(instrument_id,side,quantity,price,fees,executed_at,external_ref) VALUES(?,?,?,?,?,?,?) ON CONFLICT(external_ref) DO NOTHING", (iid,'buy',1,100,0,'2026-01-01T10:00:00Z','dup'))
assert conn.execute("SELECT COUNT(*) FROM transactions WHERE external_ref='dup'").fetchone()[0] == 1

try:
    conn.execute("INSERT INTO transactions(instrument_id,side,quantity,price,fees,executed_at) VALUES(?,?,?,?,?,?)", (iid,'buy',0,100,0,'2026-01-01T10:00:00Z'))
except sqlite3.IntegrityError:
    pass
else:
    raise AssertionError('quantity CHECK did not reject zero')

print(f'Phase 1 DB checks passed: schema v{version}, {len(expected)} required tables, indexes and constraints verified.')
