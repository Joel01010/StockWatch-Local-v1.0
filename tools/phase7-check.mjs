import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';

const root = process.cwd();
const read = (p) => fs.readFileSync(path.join(root,p),'utf8');
const exists = (p) => fs.existsSync(path.join(root,p));

for (const file of [
  'src-tauri/src/reliability/mod.rs','src-tauri/migrations/0006_reliability.sql',
  'tests/reliability_reference.test.mjs','tools/phase7-db-check.py','tools/phase7-soak.py','docs/RELIABILITY_HARDENING.md'
]) assert.ok(exists(file), `missing ${file}`);

const pkg=JSON.parse(read('package.json'));
const tauriVersion=JSON.parse(read('src-tauri/tauri.conf.json')).version;
assert.ok(pkg.version && tauriVersion, 'release versions must be present');
assert.match(read('src-tauri/Cargo.toml'), new RegExp(`version = \"${tauriVersion.replaceAll('.', '\\.') }\"`));

const storage=read('src-tauri/src/storage/mod.rs');
assert.match(storage,/MIGRATION_0006/);
assert.match(storage,/SCHEMA_VERSION: i64 = 6/);
const repo=read('src-tauri/src/storage/repository.rs');
assert.match(repo,/connection\.transaction\(\)/, 'migrations must use a transaction');
assert.match(repo,/if current_schema < SCHEMA_VERSION/);
assert.match(repo,/if \*version > current_schema/);
assert.match(repo,/PRAGMA journal_mode=WAL/);
assert.match(repo,/PRAGMA synchronous=NORMAL/);
assert.match(repo,/PRAGMA wal_checkpoint\(TRUNCATE\)/);
assert.match(repo,/DELETE FROM quote_history/);
assert.match(repo,/DELETE FROM performance_snapshots/);
assert.match(repo,/DELETE FROM alert_events/);

const rel=read('src-tauri/src/reliability/mod.rs');
for (const token of ['VACUUM INTO','PRAGMA quick_check','stockwatch.restore.pending.db','pre_migration','stockwatch.corrupt','sync_all','prune_backups','spawn_maintenance_worker']) assert.ok(rel.includes(token),`reliability missing ${token}`);
assert.match(rel,/schema_version > SCHEMA_VERSION/);
assert.match(rel,/verified/);

const settings=read('src-tauri/src/settings/mod.rs');
for (const token of ['backup_interval_hours','backup_keep_count','quote_history_retention_days','performance_retention_days','maintenance_interval_minutes','event_log_max_bytes','event_log_keep_files']) assert.ok(settings.includes(token),`settings missing ${token}`);

const scheduler=read('src-tauri/src/scheduler/mod.rs');
assert.match(scheduler,/struct WakeDetector/);
const market=read('src-tauri/src/market/service.rs');
assert.match(market,/wake\.observe/);
assert.match(market,/note_wake_recovery/);
assert.match(market,/record_refresh_duration/);
assert.match(market,/market_refresh_failed/);

const commands=read('src-tauri/src/commands.rs');
for (const name of ['reliability_status','run_maintenance_now','create_backup_now','stage_restore_backup']) assert.match(commands,new RegExp(`fn ${name}\\b`));
assert.doesNotMatch(commands,/provider_symbol:\s*String,\s*provider_symbol:\s*String/s,'duplicate provider_symbol parameter regression');

// Crude duplicate-name audit for multiline Rust command signatures; catches the Phase 6 defect class.
for (const match of commands.matchAll(/pub\s+(?:async\s+)?fn\s+(\w+)\s*\(([^)]*)\)/gs)) {
  const [,name,args]=match;
  const names=[];
  for (const part of args.split(',')) {
    const m=part.trim().match(/^(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:/);
    if (m) names.push(m[1]);
  }
  const dup=names.find((n,i)=>names.indexOf(n)!==i);
  assert.equal(dup,undefined,`duplicate parameter ${dup} in ${name}`);
}

for (const file of ['src-tauri/src/commands.rs','src-tauri/src/runtime/mod.rs','src-tauri/src/settings/mod.rs']) {
  const source=read(file);
  for (const match of source.matchAll(/struct\s+(\w+)\s*\{([^}]*)\}/gs)) {
    const [,name,body]=match;
    const fields=[...body.matchAll(/(?:pub\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:/g)].map(m=>m[1]);
    const dup=fields.find((f,i)=>fields.indexOf(f)!==i);
    assert.equal(dup,undefined,`duplicate field ${dup} in ${name} (${file})`);
  }
}

const lib=read('src-tauri/src/lib.rs');
assert.match(lib,/prepare_database\(&app_data\)/);
assert.match(lib,/spawn_maintenance_worker/);
assert.match(lib,/with_logger\(maintenance\.logger\(\)\)/);

const view=read('src/views/SettingsView.svelte');
for (const token of ['Reliability & local data','Back up now','Run maintenance','Stage restore']) assert.ok(view.includes(token));
console.log('PASS: Phase 7 hardening structure and source invariants');
