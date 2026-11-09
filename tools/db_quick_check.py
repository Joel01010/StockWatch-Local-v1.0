import sqlite3, sys
p = sys.argv[1]
c = sqlite3.connect("file:" + p + "?mode=ro", uri=True)
print("quick_check=" + str(c.execute("pragma quick_check").fetchone()[0]))
t = c.execute("select count(*) from sqlite_master where type='table' and name='meta'").fetchone()[0]
schema = c.execute("select value from meta where key='schema_version'").fetchone()[0] if t else "no-meta"
print("schema_version=" + str(schema))
c.close()
