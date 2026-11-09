import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(new URL('..', import.meta.url).pathname);
const required = [
  'START_HERE.cmd',
  'START_FULL_WINDOWS_TEST.cmd',
  'RUN_DIAGNOSTICS_ONLY.cmd',
  'scripts/BOOTSTRAP_RUNNER.ps1',
  'scripts/RUN_ALL_AUTOMATED_TESTS.ps1',
  'scripts/PREFLIGHT_WINDOWS.ps1',
  'scripts/COLLECT_SUPPORT_BUNDLE.ps1',
  'scripts/AI_OPERATOR_INSTRUCTIONS.txt',
  'scripts/lib/TestHarness.Common.ps1',
  'scripts/diagnostics/DIAG_TOOLCHAIN.ps1',
  'scripts/diagnostics/DIAG_NETWORK.ps1',
  'scripts/diagnostics/DIAG_MARKET_DATA.ps1',
  'scripts/diagnostics/DIAG_WEBVIEW2.ps1',
  'scripts/diagnostics/DIAG_BUILD.ps1',
  'scripts/diagnostics/DIAG_RUNTIME.ps1',
  'scripts/diagnostics/DIAG_TRAY_WEBVIEW.ps1',
  'scripts/diagnostics/DIAG_AUTOSTART.ps1',
  'scripts/diagnostics/DIAG_NOTIFICATIONS.ps1',
  'scripts/diagnostics/DIAG_DATABASE.ps1',
  'scripts/diagnostics/DIAG_RESOURCES.ps1',
  'scripts/diagnostics/DIAG_SECURITY.ps1',
  'scripts/diagnostics/DIAG_INSTALLER.ps1',
  'scripts/diagnostics/DIAG_SLEEP_RESUME.ps1',
];

const failures = [];
for (const rel of required) {
  const file = path.join(root, rel);
  if (!fs.existsSync(file)) failures.push(`missing ${rel}`);
  else if (fs.statSync(file).size < 20) failures.push(`suspiciously empty ${rel}`);
}

const bootstrap = fs.readFileSync(path.join(root, 'scripts/BOOTSTRAP_RUNNER.ps1'), 'utf8');
if (!bootstrap.includes('RUN_ALL_AUTOMATED_TESTS.ps1') || !bootstrap.includes('DIAGNOSE_ALL.ps1')) failures.push('bootstrap lacks master/fallback path');

const master = fs.readFileSync(path.join(root, 'scripts/RUN_ALL_AUTOMATED_TESTS.ps1'), 'utf8');
for (const marker of ['PREFLIGHT_WINDOWS.ps1','QUALIFY_WINDOWS.ps1','COLLECT_SUPPORT_BUNDLE.ps1','RETURN_THIS_FILE','SUPPORT_BUNDLE']) {
  if (!master.includes(marker)) failures.push(`master missing marker ${marker}`);
}
if (/Read-Host|Pause\b|choice\.exe/i.test(master)) failures.push('master must not prompt interactively');
if (/DIAG_SLEEP_RESUME\.ps1/.test(master)) failures.push('master must not auto-run sleep/resume diagnostic');

const start = fs.readFileSync(path.join(root, 'START_HERE.cmd'), 'utf8');
if (!/BOOTSTRAP_RUNNER\.ps1/i.test(start)) failures.push('START_HERE does not call bootstrap runner');
if (/START_FULL_WINDOWS_TEST/i.test(start)) failures.push('safe START_HERE unexpectedly calls full install test');

const rustLib = fs.readFileSync(path.join(root, 'src-tauri/src/lib.rs'), 'utf8');
if (!rustLib.includes('STOCKWATCH_QUALIFICATION_MODE')) failures.push('Rust startup lacks qualification-mode autostart isolation');
const common = fs.readFileSync(path.join(root, 'scripts/lib/TestHarness.Common.ps1'), 'utf8');
if (!common.includes("STOCKWATCH_QUALIFICATION_MODE='1'")) failures.push('isolated process helper does not enable qualification mode');

const installed = fs.readFileSync(path.join(root, 'scripts/QUALIFY_INSTALLED_WINDOWS.ps1'), 'utf8');
for (const marker of ['preExistingInstall','Refusing to overwrite','Invoke-Uninstall','KeepInstalled']) {
  if (!installed.includes(marker)) failures.push(`installed qualifier missing safety marker ${marker}`);
}

const support = fs.readFileSync(path.join(root, 'scripts/COLLECT_SUPPORT_BUNDLE.ps1'), 'utf8');
if (/Get-ChildItem\s+Env:|gci\s+env:/i.test(support)) failures.push('support bundle must not dump full environment');


const safePs1 = [
  'scripts/BOOTSTRAP_RUNNER.ps1',
  'scripts/RUN_ALL_AUTOMATED_TESTS.ps1','scripts/PREFLIGHT_WINDOWS.ps1','scripts/COLLECT_SUPPORT_BUNDLE.ps1','scripts/DIAGNOSE_ALL.ps1',
  ...required.filter(x => x.startsWith('scripts/diagnostics/') && !x.endsWith('DIAG_SLEEP_RESUME.ps1'))
];
for (const rel of safePs1) {
  const text = fs.readFileSync(path.join(root, rel), 'utf8');
  if (/Read-Host|Pause\b|choice\.exe/i.test(text)) failures.push(`${rel} contains an interactive prompt`);
  if (/Set-MpPreference|netsh\s+winhttp\s+set|reg\.exe\s+add/i.test(text)) failures.push(`${rel} contains forbidden system reconfiguration`);
}

if (failures.length) {
  console.error('FAIL: test harness checks');
  for (const f of failures) console.error(` - ${f}`);
  process.exit(1);
}
console.log(`PASS: test harness structure (${required.length} required artifacts)`);
