import sqlite3, tempfile, time
from pathlib import Path
CYCLES=25000
INTERVAL_MS=30_000
BUCKET_MS=300_000
RETENTION_MS=365*24*60*60*1000
with tempfile.TemporaryDirectory() as td:
    db=Path(td)/'soak.db'; c=sqlite3.connect(db)
    c.executescript('create table q(t integer primary key,p real); create table s(bucket integer primary key,v real);')
    start=time.perf_counter()
    end=0
    for i in range(CYCLES):
        t=i*INTERVAL_MS; end=t
        c.execute('insert into q(t,p) values(?,?)',(t,100+(i%100)/10))
        b=(t//BUCKET_MS)*BUCKET_MS
        c.execute('insert into s(bucket,v) values(?,?) on conflict(bucket) do update set v=excluded.v',(b,1000+i/100))
        if i and i%1000==0:
            cutoff=max(0,t-RETENTION_MS); c.execute('delete from q where t < ?',(cutoff,)); c.commit()
    c.commit()
    q=c.execute('select count(*) from q').fetchone()[0]; s=c.execute('select count(*) from s').fetchone()[0]
    integrity=c.execute('pragma quick_check').fetchone()[0]; c.close()
    elapsed=time.perf_counter()-start
    assert integrity=='ok'; assert s <= CYCLES//10+1; assert q<=CYCLES
    print(f'PASS: simulated {CYCLES} refreshes in {elapsed:.3f}s; quote_rows={q}; snapshot_rows={s}; integrity={integrity}')
