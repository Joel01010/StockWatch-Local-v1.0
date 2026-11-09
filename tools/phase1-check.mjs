import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';

const root = path.resolve(import.meta.dirname, '..');
const required = [
  'src-tauri/src/portfolio/model.rs',
  'src-tauri/src/portfolio/engine.rs',
  'src-tauri/src/imports/mod.rs',
  'src-tauri/src/storage/repository.rs',
  'src-tauri/migrations/0002_portfolio_core.sql',
  'tests/portfolio_reference.test.mjs',
  'tests/fixtures/transactions.csv',
  'tests/fixtures/holdings.csv',
  'examples/current-holdings.csv',
  'src-tauri/src/commands.rs',
];

for (const rel of required) {
  assert.ok(fs.existsSync(path.join(root, rel)), `missing ${rel}`);
}

const cargo = fs.readFileSync(path.join(root, 'src-tauri/Cargo.toml'), 'utf8');
for (const dep of ['rust_decimal', 'csv', 'rusqlite', 'tauri']) {
  assert.match(cargo, new RegExp(`^${dep}\\s*=`, 'm'), `Cargo dependency ${dep} missing`);
}

const engine = fs.readFileSync(path.join(root, 'src-tauri/src/portfolio/engine.rs'), 'utf8');
assert.match(engine, /Oversell/);
assert.match(engine, /average_price/);
assert.match(engine, /realized_gross_pnl/);
assert.match(engine, /sort_by/);

const repo = fs.readFileSync(path.join(root, 'src-tauri/src/storage/repository.rs'), 'utf8');
assert.match(repo, /transaction\(\)/, 'atomic SQLite transaction missing');
assert.match(repo, /ON CONFLICT\(external_ref\) DO NOTHING/, 'idempotent import handling missing');
assert.match(repo, /derive_portfolio\(&load_trades/, 'ledger validation before commit missing');

const importSource = fs.readFileSync(path.join(root, 'src-tauri/src/imports/mod.rs'), 'utf8');
assert.match(importSource, /parse_transactions_csv/);
assert.match(importSource, /parse_holdings_csv/);
assert.match(importSource, /RFC3339|Rfc3339/);

console.log(`Phase 1 structural checks passed (${required.length} required artifacts).`);

const lib = fs.readFileSync(path.join(root, 'src-tauri/src/lib.rs'), 'utf8');
assert.match(lib, /calculate_portfolio/);
assert.match(lib, /preview_opening_holdings/);
