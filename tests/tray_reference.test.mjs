import test from 'node:test';
import assert from 'node:assert/strict';

function effectiveSymbols(settings) {
  const max = Math.max(1, Math.min(20, settings.tray.maxSymbols));
  const seen = new Set();
  const out = [];
  for (const raw of settings.traySymbols) {
    const symbol = raw.trim().toUpperCase();
    if (!symbol || seen.has(symbol)) continue;
    seen.add(symbol); out.push(symbol);
    if (out.length >= max) break;
  }
  return out;
}

function action(id) {
  if (id === 'action:dashboard') return { kind: 'dashboard' };
  if (id === 'action:settings') return { kind: 'settings' };
  if (id === 'action:refresh') return { kind: 'refresh' };
  if (id === 'action:alerts') return { kind: 'alerts' };
  if (id === 'action:exit') return { kind: 'exit' };
  if (id.startsWith('symbol:') && id.length > 7) return { kind: 'symbol', symbol: id.slice(7) };
  return null;
}

function menu(snapshot, settings) {
  const entries = [
    {id:'status:product', kind:'display', enabled:false},
    {id:'status:market', kind:'display', enabled:false},
  ];
  const symbols = effectiveSymbols(settings);
  const holdings = symbols.length ? symbols.map(s => snapshot.holdings.find(h => h.symbol.toUpperCase() === s)).filter(Boolean) : snapshot.holdings.slice(0, settings.tray.maxSymbols);
  if (!holdings.length) entries.push({id:'status:no-holdings', kind:'display', enabled:false});
  else for (const h of holdings) entries.push({id:`symbol:${h.symbol.toUpperCase()}`, kind:'action', enabled:true});
  entries.push({id:'action:alerts', kind:'action', enabled:true});
  entries.push({id:'action:refresh', kind:'action', enabled:!snapshot.refreshInFlight});
  entries.push({id:'action:dashboard', kind:'action', enabled:true});
  entries.push({id:'action:settings', kind:'action', enabled:true});
  entries.push({id:'action:exit', kind:'action', enabled:true});
  return entries;
}

const settings = {
  traySymbols: [' demotech ', 'DEMOTECH', 'demoetf', 'demobank'],
  tray: { maxSymbols: 2 }
};
const snapshot = { refreshInFlight:false, holdings:[{symbol:'DEMOTECH'},{symbol:'DEMOETF'},{symbol:'DEMOBANK'}] };

test('configured tray symbols normalize, deduplicate and honor maximum', () => {
  assert.deepEqual(effectiveSymbols(settings), ['DEMOTECH','DEMOETF']);
});

test('menu emits only selected holdings and primary commands', () => {
  const ids = menu(snapshot, settings).map(x => x.id);
  assert(ids.includes('symbol:DEMOTECH'));
  assert(ids.includes('symbol:DEMOETF'));
  assert(!ids.includes('symbol:DEMOBANK'));
  for (const id of ['action:alerts','action:refresh','action:dashboard','action:settings','action:exit']) assert(ids.includes(id));
});

test('refresh action becomes disabled while one is in flight', () => {
  const m = menu({...snapshot, refreshInFlight:true}, settings);
  assert.equal(m.find(x => x.id === 'action:refresh').enabled, false);
});

test('menu parser ignores display ids and parses symbol commands deterministically', () => {
  assert.equal(action('status:market'), null);
  assert.deepEqual(action('symbol:DEMOTECH'), {kind:'symbol', symbol:'DEMOTECH'});
  assert.deepEqual(action('action:exit'), {kind:'exit'});
});

test('empty selection falls back to bounded first holdings', () => {
  const m = menu(snapshot, {traySymbols:[], tray:{maxSymbols:2}});
  const symbols = m.filter(x => x.id.startsWith('symbol:')).map(x=>x.id);
  assert.deepEqual(symbols, ['symbol:DEMOTECH','symbol:DEMOETF']);
});

test('refresh gate rejects duplicate work and becomes eligible after completion', () => {
  const state = { refreshInFlight: false, error: null };
  const begin = () => { if (state.refreshInFlight) return false; state.refreshInFlight = true; state.error = null; return true; };
  const fail = (message) => { state.refreshInFlight = false; state.error = message; };
  assert.equal(begin(), true);
  assert.equal(begin(), false);
  fail('provider not connected');
  assert.equal(state.refreshInFlight, false);
  assert.equal(begin(), true);
});
