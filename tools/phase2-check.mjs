import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const mustExist = [
  'src-tauri/src/runtime/mod.rs',
  'src-tauri/src/tray/mod.rs',
  'src-tauri/src/tray/model.rs',
  'src-tauri/src/tray/native.rs',
  'src-tauri/icons/32x32.png',
  'src-tauri/icons/128x128.png',
  'src-tauri/icons/icon.ico',
  'tests/tray_reference.test.mjs',
  'docs/TRAY_RUNTIME.md'
];
let failures=[];
for (const file of mustExist) if (!fs.existsSync(path.join(root,file))) failures.push(`missing ${file}`);
const lib = fs.readFileSync(path.join(root,'src-tauri/src/lib.rs'),'utf8');
const native = fs.readFileSync(path.join(root,'src-tauri/src/tray/native.rs'),'utf8');
const settings = fs.readFileSync(path.join(root,'src-tauri/src/settings/mod.rs'),'utf8');
const conf = JSON.parse(fs.readFileSync(path.join(root,'src-tauri/tauri.conf.json'),'utf8'));
if (!lib.includes('install_native_tray')) failures.push('native tray is not installed during setup');
if (!native.includes('.show_menu_on_left_click(false)')) failures.push('right-click-only menu behavior missing');
if (!native.includes('WebviewWindowBuilder::new')) failures.push('on-demand dashboard creation missing');
if (!native.includes('std::thread::spawn')) failures.push('dashboard creation is not deferred outside synchronous tray events');
if (!native.includes('CloseRequested') || !native.includes('api.prevent_close()')) failures.push('hide-to-tray close policy adapter missing');
if (!native.includes('set_menu(Some(menu))')) failures.push('reactive tray menu refresh missing');
if (!settings.includes('DestroyToTray')) failures.push('WebView-destroying close policy missing');
if (!Array.isArray(conf.app.windows) || conf.app.windows.length !== 0) failures.push('dashboard must not be pre-created at startup');
if (failures.length) {
  console.error(`Phase 2 structural check FAILED (${failures.length})`);
  for (const f of failures) console.error(` - ${f}`);
  process.exit(1);
}
console.log(`Phase 2 structural check PASS (${mustExist.length} required artifacts + architecture invariants)`);
