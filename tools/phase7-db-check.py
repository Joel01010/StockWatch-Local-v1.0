from pathlib import Path
import sqlite3, tempfile, shutil, os, json, datetime
ROOT=Path(__file__).resolve().parents[1]
MIGRATIONS=[ROOT/'src-tauri/migrations'/f'000{i}_{name}.sql' for i,name in [
 (1,'initial'),(2,'portfolio_core'),(3,'market_data'),(4,'alerts_settings'),(5,'analytics'),(6,'reliability')]]

def apply(conn):
    conn.execute('PRAGMA foreign_keys=ON')
    conn.execute('BEGIN')
    try:
        for p in MIGRATIONS: conn.executescript(p.read_text())
        conn.commit()
    except Exception:
        conn.rollback(); raise

def quick(path):
    c=sqlite3.connect(path); r=c.execute('PRAGMA quick_check').fetchone()[0]; c.close(); return r

with tempfile.TemporaryDirectory() as td:
    td=Path(td); db=td/'stockwatch.db'; c=sqlite3.connect(db); apply(c)
    v=int(c.execute("select value from schema_meta where key='schema_version'").fetchone()[0]); assert v==6
    tables={r[0] for r in c.execute("select name from sqlite_master where type='table'")}
    assert {'reliability_state','performance_snapshots','alert_events','quote_history'}<=tables
    c.execute("insert into instruments(exchange,symbol) values('NSE','DEMOTECH')"); iid=c.execute('select id from instruments').fetchone()[0]
    c.execute("insert into alerts(id,instrument_id,condition_json) values('a1',?,?)",(iid,'{"kind":"price_above","value":300}'))
    old='2020-01-01T00:00:00Z'; new='2026-11-08T12:00:00Z'
    c.execute("insert into quote_history(instrument_id,price,observed_at,source) values(?,?,?,?)",(iid,100,old,'test'))
    c.execute("insert into quote_history(instrument_id,price,observed_at,source) values(?,?,?,?)",(iid,200,new,'test'))
    c.execute("insert into performance_snapshots(bucket_unix_ms,observed_at,portfolio_value,open_cost_basis,realized_gross_pnl,fees_paid,investment_income,net_external_flow,source) values(1,?,?,?,?,?,?,?,?)",(old,100,90,0,0,0,0,'test'))
    c.execute("insert into performance_snapshots(bucket_unix_ms,observed_at,portfolio_value,open_cost_basis,realized_gross_pnl,fees_paid,investment_income,net_external_flow,source) values(2,?,?,?,?,?,?,?,?)",(new,200,90,0,0,0,0,'test'))
    c.execute("insert into alert_events(alert_id,fired_at,title,body,delivery_status) values('a1',?,'x','x','recorded_only')",(old,))
    c.execute("insert into alert_events(alert_id,fired_at,title,body,delivery_status) values('a1',?,'x','x','recorded_only')",(new,))
    cutoff='2025-01-01T00:00:00Z'
    q=c.execute('delete from quote_history where observed_at < ?',(cutoff,)).rowcount
    p=c.execute('delete from performance_snapshots where observed_at < ?',(cutoff,)).rowcount
    a=c.execute('delete from alert_events where fired_at < ?',(cutoff,)).rowcount
    assert (q,p,a)==(1,1,1); c.commit(); c.close()
    assert quick(db)=='ok'
    backup=td/'backup.db'; conn=sqlite3.connect(db); conn.execute(f"VACUUM INTO '{str(backup).replace(chr(39), chr(39)*2)}'"); conn.close()
    assert backup.exists() and quick(backup)=='ok'
    staged=td/'stockwatch.restore.pending.db'; shutil.copy2(backup,staged); assert quick(staged)=='ok'
    recovered=td/'restored.db'; os.replace(staged,recovered); assert quick(recovered)=='ok'
print('PASS: schema v6, retention, quick_check, VACUUM backup and staged-restore primitives')
