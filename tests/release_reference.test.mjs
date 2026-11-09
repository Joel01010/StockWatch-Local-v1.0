import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';

const lib = fs.readFileSync('src-tauri/src/lib.rs','utf8');
const cargo = fs.readFileSync('src-tauri/Cargo.toml','utf8');
const conf = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json','utf8'));
const pkg = JSON.parse(fs.readFileSync('package.json','utf8'));

test('release versions are synchronized', () => {
  assert.equal(pkg.version, '1.0.0-rc.2');
  assert.equal(conf.version, pkg.version);
  assert.match(cargo, /version = "1\.0\.0-rc\.2"/);
});

test('normal window close keeps the tray process alive but explicit coded exit is allowed', () => {
  assert.match(lib, /RunEvent::ExitRequested/);
  assert.match(lib, /if code\.is_none\(\)/);
  assert.match(lib, /api\.prevent_exit\(\)/);
  assert.match(lib, /exit_handle\.exit\(0\)/);
});

test('release startup remains WebView-free until requested', () => {
  assert.deepEqual(conf.app.windows, []);
  assert.match(lib, /STOCKWATCH_QUALIFICATION_WINDOW/);
});

test('native autostart is implemented in the Rust backend', () => {
  assert.match(cargo, /tauri-plugin-autostart/);
  assert.match(lib, /autolaunch\(\)\.enable\(\)/);
  assert.match(lib, /autolaunch\(\)\.disable\(\)/);
});

test('qualification mode cannot overwrite the real autostart preference during isolated launches', () => {
  assert.match(lib, /STOCKWATCH_QUALIFICATION_MODE/);
  assert.match(lib, /if !qualification_mode/);
});
