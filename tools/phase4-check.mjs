import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const required = [
  'src-tauri/src/market/model.rs', 'src-tauri/src/market/provider.rs', 'src-tauri/src/market/freshness.rs',
  'src-tauri/src/market/yahoo.rs', 'src-tauri/src/market/service.rs', 'src-tauri/migrations/0003_market_data.sql',
  'tests/market_reference.test.mjs', 'tools/phase4-db-check.py', 'docs/MARKET_DATA.md', 'src/views/MarketMonitor.svelte'
];
const failures=[];
for (const rel of required) if (!fs.existsSync(path.join(root,rel))) failures.push(`missing ${rel}`);
const cargo = fs.readFileSync(path.join(root,'src-tauri/Cargo.toml'),'utf8');
const market = fs.readFileSync(path.join(root,'src-tauri/src/market/mod.rs'),'utf8');
const yahoo = fs.readFileSync(path.join(root,'src-tauri/src/market/yahoo.rs'),'utf8');
const service = fs.readFileSync(path.join(root,'src-tauri/src/market/service.rs'),'utf8');
const scheduler = fs.readFileSync(path.join(root,'src-tauri/src/scheduler/mod.rs'),'utf8');
const storage = fs.readFileSync(path.join(root,'src-tauri/src/storage/repository.rs'),'utf8');
const commands = fs.readFileSync(path.join(root,'src-tauri/src/commands.rs'),'utf8');
const app = fs.readFileSync(path.join(root,'src/App.svelte'),'utf8');
const lib = fs.readFileSync(path.join(root,'src-tauri/src/lib.rs'),'utf8');
for (const dep of ['reqwest','async-trait','urlencoding']) if (!new RegExp(`^${dep}\\s*=`, 'm').test(cargo)) failures.push(`missing Cargo dependency ${dep}`);
if (!market.includes('MarketDataProvider') || !market.includes('MarketDataService')) failures.push('provider/service exports missing');
if (!yahoo.includes('QuoteFreshness::Delayed')) failures.push('public adapter must explicitly classify data as delayed');
if (yahoo.includes('QuoteFreshness::Live')) failures.push('delayed adapter must never label its own data live');
if (!yahoo.includes('query1.finance.yahoo.com/v8/finance/chart/')) failures.push('Yahoo chart adapter endpoint missing');
if (!service.includes('upsert_quotes_atomic') || !service.includes('refresh_runtime_from_cache')) failures.push('cache persistence/runtime hydration missing');
if (!service.includes('BackoffState') || !scheduler.includes('saturating_mul')) failures.push('bounded failure backoff missing');
if (!scheduler.includes('09:15') || !scheduler.includes('15:30')) failures.push('NSE session estimate documentation missing');
if (!storage.includes('quote_targets') || !storage.includes('watchlist_items')) failures.push('holdings/watchlist target union missing');
if (!storage.includes('market_provider_state')) failures.push('provider health persistence missing');
if (!commands.includes('MarketDataService') || !commands.includes('request_refresh')) failures.push('manual refresh command is not connected to market service');
if (!app.includes('MarketMonitor')) failures.push('market monitor UI route missing');
if (!lib.includes('spawn_market_worker') || !lib.includes('refresh_on_startup')) failures.push('background market worker/startup refresh missing');
if (failures.length) {
  console.error(`Phase 4 structural check FAILED (${failures.length})`);
  for (const failure of failures) console.error(` - ${failure}`);
  process.exit(1);
}
console.log(`Phase 4 structural check PASS (${required.length} required artifacts + market-data invariants)`);
