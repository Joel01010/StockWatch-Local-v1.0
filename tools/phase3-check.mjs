import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const required = [
  'src/views/Overview.svelte', 'src/views/Holdings.svelte', 'src/views/HoldingDetail.svelte',
  'src/views/MiniMode.svelte', 'src/components/PerformanceChart.svelte', 'src/components/HoldingsTable.svelte',
  'src/components/Sidebar.svelte', 'src/components/Topbar.svelte', 'src/lib/bridge.ts', 'src/lib/routing.ts',
  'tests/dashboard_reference.test.mjs', 'docs/DASHBOARD_UI.md'
];
const failures=[];
for (const rel of required) if (!fs.existsSync(path.join(root,rel))) failures.push(`missing ${rel}`);
const app = fs.readFileSync(path.join(root,'src/App.svelte'),'utf8');
const bridge = fs.readFileSync(path.join(root,'src/lib/bridge.ts'),'utf8');
const commands = fs.readFileSync(path.join(root,'src-tauri/src/commands.rs'),'utf8');
const native = fs.readFileSync(path.join(root,'src-tauri/src/tray/native.rs'),'utf8');
const model = fs.readFileSync(path.join(root,'src-tauri/src/tray/model.rs'),'utf8');
const pkg = JSON.parse(fs.readFileSync(path.join(root,'package.json'),'utf8'));
const conf = JSON.parse(fs.readFileSync(path.join(root,'src-tauri/tauri.conf.json'),'utf8'));
if (!app.includes('MiniMode') || !app.includes('HoldingDetail')) failures.push('dashboard route surfaces missing');
if (!bridge.includes('invoke<DashboardSnapshot>("dashboard_snapshot")')) failures.push('local Rust dashboard bridge missing');
if (!commands.includes('PortfolioRepository::open') || !commands.includes('derive_portfolio')) failures.push('dashboard snapshot is not sourced from local portfolio repository');
if (!commands.includes('QuoteFreshness::Offline')) failures.push('missing quote state is not explicitly offline');
if (!native.includes('index.html#/mini') || !native.includes('.decorations(false)') || !native.includes('.skip_taskbar(true)')) failures.push('native mini window configuration missing');
if (!model.includes('Quick View') || !model.includes('OpenMiniMode')) failures.push('tray quick-view action missing');
if (!native.includes('safe_route') || !native.includes("window.location.hash")) failures.push('existing dashboard route handoff missing or unsanitized');
if (!Array.isArray(conf.app.windows) || conf.app.windows.length !== 0) failures.push('dashboard must remain on-demand, not pre-created');
const deps = {...pkg.dependencies,...pkg.devDependencies};
for (const unwanted of ['chart.js','recharts','highcharts','echarts']) if (deps[unwanted]) failures.push(`heavy chart dependency present: ${unwanted}`);
if (failures.length) {
  console.error(`Phase 3 structural check FAILED (${failures.length})`);
  for (const failure of failures) console.error(` - ${failure}`);
  process.exit(1);
}
console.log(`Phase 3 structural check PASS (${required.length} required artifacts + UI/runtime invariants)`);
