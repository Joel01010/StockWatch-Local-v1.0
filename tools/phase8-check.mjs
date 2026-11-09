import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const read = p => fs.readFileSync(path.join(root,p),'utf8');
const pkg = JSON.parse(read('package.json'));
const tauri = JSON.parse(read('src-tauri/tauri.conf.json'));
const cargo = read('src-tauri/Cargo.toml');
const lib = read('src-tauri/src/lib.rs');
const commands = read('src-tauri/src/commands.rs');
const tray = read('src-tauri/src/tray/native.rs');
const required = [
  'scripts/QUALIFY_WINDOWS.ps1',
  'scripts/QUALIFY_INSTALLED_WINDOWS.ps1',
  'scripts/PACKAGE_WINDOWS_RC.ps1',
  'docs/WINDOWS_QUALIFICATION.md',
];
for (const file of required) {
  if (!fs.existsSync(path.join(root,file))) throw new Error(`Missing Phase 8 artifact: ${file}`);
}
if (pkg.version !== '1.0.0-rc.2') throw new Error('package version mismatch');
if (tauri.version !== '1.0.0-rc.2') throw new Error('tauri version mismatch');
if (!cargo.includes('version = "1.0.0-rc.2"')) throw new Error('cargo version mismatch');
if (!cargo.includes('tauri-plugin-autostart = "2"')) throw new Error('native autostart plugin missing');
if (!lib.includes('tauri_plugin_autostart::init')) throw new Error('autostart plugin not initialized');
if (!commands.includes('app.autolaunch()')) throw new Error('settings do not apply native autostart');
if (!lib.includes('RunEvent::ExitRequested') || !lib.includes('code.is_none()') || !lib.includes('api.prevent_exit()')) throw new Error('tray-first process lifetime guard missing');
if (!lib.includes('STOCKWATCH_QUALIFICATION_WINDOW') || !lib.includes('STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS')) throw new Error('native qualification hooks missing');
if (!tray.includes('request_qualification_window')) throw new Error('qualification window hook missing');
if (!Array.isArray(tauri.bundle.targets) || !tauri.bundle.targets.includes('nsis')) throw new Error('NSIS target missing');
if (tauri.app.windows.length !== 0) throw new Error('startup must remain tray-only with no configured WebView');
console.log('PASS: Phase 8 release/Windows-qualification source invariants');
