from pathlib import Path
import sqlite3, json
root=Path(__file__).resolve().parents[1]
conn=sqlite3.connect(':memory:')
for name in ['0001_initial.sql','0002_portfolio_core.sql','0003_market_data.sql','0004_alerts_settings.sql']:
    conn.executescript((root/'src-tauri/migrations'/name).read_text())
version=conn.execute("SELECT value FROM schema_meta WHERE key='schema_version'").fetchone()[0]
assert version=='4',version
tables={r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table'")}
for t in {'alerts','alert_rule_meta','alert_events','settings'}: assert t in tables,t
conn.execute("INSERT INTO instruments(exchange,symbol,name) VALUES('NSE','DEMOTECH','Demo Technology')")
iid=conn.execute("SELECT id FROM instruments WHERE symbol='DEMOTECH'").fetchone()[0]
cond=json.dumps({'kind':'price_above','value':300.0})
conn.execute("INSERT INTO alerts(id,instrument_id,enabled,condition_json,cooldown_seconds,reset_margin) VALUES(?,?,?,?,?,?)",('a1',iid,1,cond,900,2.0))
conn.execute("INSERT INTO alert_rule_meta(alert_id,exchange,repeat_mode,market_hours_only,armed,last_value,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?)",('a1','NSE','crossing',1,1,299.0,'2026-11-08T10:00:00Z','2026-11-08T10:00:00Z'))
conn.execute("UPDATE alert_rule_meta SET armed=0,last_value=301 WHERE alert_id='a1'")
assert conn.execute("SELECT armed,last_value FROM alert_rule_meta WHERE alert_id='a1'").fetchone()==(0,301.0)
conn.execute("INSERT INTO alert_events(alert_id,fired_at,metric_value,title,body,delivery_status) VALUES(?,?,?,?,?,?)",('a1','2026-11-08T10:01:00Z',301,'DEMOTECH alert','Price above','recorded_only'))
assert conn.execute("SELECT COUNT(*) FROM alert_events").fetchone()[0]==1
settings={'notifications':{'enabled':True}}
conn.execute("INSERT INTO settings(key,value_json,updated_at) VALUES('app_settings',?,?)",(json.dumps(settings),'2026-11-08T10:00:00Z'))
assert json.loads(conn.execute("SELECT value_json FROM settings WHERE key='app_settings'").fetchone()[0])['notifications']['enabled'] is True
# idempotent migration re-run
conn.executescript((root/'src-tauri/migrations'/'0004_alerts_settings.sql').read_text())
assert conn.execute("SELECT COUNT(*) FROM alert_rule_meta").fetchone()[0]==1
print(f'Phase 5 DB checks passed: schema v{version}, alert state/history/settings persistence and idempotent v4 migration verified.')
