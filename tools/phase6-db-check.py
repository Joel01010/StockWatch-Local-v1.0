from pathlib import Path
import sqlite3
root=Path(__file__).resolve().parents[1]
conn=sqlite3.connect(':memory:')
for name in ['0001_initial.sql','0002_portfolio_core.sql','0003_market_data.sql','0004_alerts_settings.sql','0005_analytics.sql']:
    conn.executescript((root/'src-tauri/migrations'/name).read_text())
version=conn.execute("SELECT value FROM schema_meta WHERE key='schema_version'").fetchone()[0]
assert version=='5',version
tables={r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table'")}
for t in {'cash_flows','performance_snapshots','performance_holding_snapshots','analytics_preferences'}: assert t in tables,t
conn.execute("INSERT INTO cash_flows(kind,amount,occurred_at,note) VALUES('deposit',1000,'2026-11-08T10:00:00Z','capital')")
conn.execute("INSERT INTO cash_flows(kind,amount,occurred_at,note) VALUES('dividend',25,'2026-11-08T10:01:00Z','income')")
assert conn.execute("SELECT SUM(CASE WHEN kind='deposit' THEN amount ELSE 0 END) FROM cash_flows").fetchone()[0]==1000
try:
    conn.execute("INSERT INTO cash_flows(kind,amount,occurred_at) VALUES('deposit',0,'2026-11-08T10:00:00Z')")
    raise AssertionError('zero cash flow should be rejected')
except sqlite3.IntegrityError: pass
conn.execute("INSERT INTO instruments(exchange,symbol,name) VALUES('NSE','DEMOTECH','Demo Technology')")
iid=conn.execute("SELECT id FROM instruments WHERE symbol='DEMOTECH'").fetchone()[0]
conn.execute("INSERT INTO performance_snapshots(bucket_unix_ms,observed_at,portfolio_value,open_cost_basis,realized_gross_pnl,fees_paid,investment_income,net_external_flow,source) VALUES(?,?,?,?,?,?,?,?,?)",(1000,'2026-11-08T10:00:00Z',2557.00,2500.01,0,1.2,25,1000,'test'))
sid=conn.execute("SELECT id FROM performance_snapshots WHERE bucket_unix_ms=1000").fetchone()[0]
conn.execute("INSERT INTO performance_holding_snapshots(snapshot_id,instrument_id,quantity,price,current_value,open_cost_basis) VALUES(?,?,?,?,?,?)",(sid,iid,2,210.00,840.00,800.01))
# Same time bucket is replaceable/upsertable by production code because the schema enforces one row per bucket.
try:
    conn.execute("INSERT INTO performance_snapshots(bucket_unix_ms,observed_at,portfolio_value,open_cost_basis,realized_gross_pnl,fees_paid,investment_income,net_external_flow,source) VALUES(?,?,?,?,?,?,?,?,?)",(1000,'2026-11-08T10:04:00Z',830,747.3,0,1.2,25,1000,'test'))
    raise AssertionError('duplicate bucket must be unique')
except sqlite3.IntegrityError: pass
conn.execute("UPDATE analytics_preferences SET benchmark_exchange='NSE',benchmark_symbol='NIFTYBEES',updated_at='2026-11-08T10:00:00Z' WHERE id=1")
assert conn.execute("SELECT benchmark_exchange,benchmark_symbol FROM analytics_preferences WHERE id=1").fetchone()==('NSE','NIFTYBEES')
conn.execute("DELETE FROM performance_snapshots WHERE id=?",(sid,))
assert conn.execute("SELECT COUNT(*) FROM performance_holding_snapshots").fetchone()[0]==0
conn.executescript((root/'src-tauri/migrations'/'0005_analytics.sql').read_text())
assert conn.execute("SELECT COUNT(*) FROM cash_flows").fetchone()[0]==2
print(f'Phase 6 DB checks passed: schema v{version}, cash-flow ledger, compact snapshots, holding snapshot cascade, benchmark preferences, and idempotent v5 migration verified.')
