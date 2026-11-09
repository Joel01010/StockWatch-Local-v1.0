from pathlib import Path
import sqlite3

root = Path(__file__).resolve().parents[1]
conn = sqlite3.connect(':memory:')
for name in ['0001_initial.sql','0002_portfolio_core.sql','0003_market_data.sql']:
    conn.executescript((root/'src-tauri/migrations'/name).read_text())

version = conn.execute("SELECT value FROM schema_meta WHERE key='schema_version'").fetchone()[0]
assert version == '3', version

tables = {r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table'")}
expected = {'schema_meta','instruments','transactions','quotes','quote_history','portfolio_snapshots','alerts','watchlists','watchlist_items','settings','import_batches','market_provider_state','instrument_provider_symbols'}
missing = expected - tables
assert not missing, missing

indexes = {r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='index'")}
for idx in {'idx_quote_history_dedupe','idx_quote_history_instrument_time','idx_transactions_instrument_time'}:
    assert idx in indexes, idx

conn.execute("INSERT INTO instruments(exchange,symbol,name) VALUES('NSE','DEMOTECH','Demo Technology')")
iid = conn.execute("SELECT id FROM instruments WHERE exchange='NSE' AND symbol='DEMOTECH'").fetchone()[0]
conn.execute("INSERT INTO quotes(instrument_id,price,change_pct,observed_at,source,freshness) VALUES(?,?,?,?,?,?)", (iid,299.3,1.85,'2026-11-08T10:00:00Z','test_delayed','delayed'))
conn.execute("INSERT INTO quotes(instrument_id,price,change_pct,observed_at,source,freshness) VALUES(?,?,?,?,?,?) ON CONFLICT(instrument_id) DO UPDATE SET price=excluded.price, observed_at=excluded.observed_at", (iid,300.0,2.0,'2026-11-08T10:01:00Z','test_delayed','delayed'))
assert conn.execute("SELECT COUNT(*) FROM quotes WHERE instrument_id=?",(iid,)).fetchone()[0] == 1
assert conn.execute("SELECT price FROM quotes WHERE instrument_id=?",(iid,)).fetchone()[0] == 300.0

conn.execute("INSERT OR IGNORE INTO quote_history(instrument_id,price,observed_at,source) VALUES(?,?,?,?)",(iid,300,'2026-11-08T10:01:00Z','test_delayed'))
conn.execute("INSERT OR IGNORE INTO quote_history(instrument_id,price,observed_at,source) VALUES(?,?,?,?)",(iid,300,'2026-11-08T10:01:00Z','test_delayed'))
assert conn.execute("SELECT COUNT(*) FROM quote_history").fetchone()[0] == 1

conn.execute("INSERT INTO market_provider_state(provider_id,consecutive_failures,last_error) VALUES('test',1,'offline')")
conn.execute("UPDATE market_provider_state SET consecutive_failures=0,last_success_at='2026-11-08T10:02:00Z',last_error=NULL WHERE provider_id='test'")
state = conn.execute("SELECT consecutive_failures,last_error FROM market_provider_state WHERE provider_id='test'").fetchone()
assert state == (0,None)

print(f'Phase 4 DB checks passed: schema v{version}, {len(expected)} required tables, quote cache/history and provider state verified.')
