import test from 'node:test';
import assert from 'node:assert/strict';

function providerSymbol(exchange, symbol) {
  const s = symbol.trim().toUpperCase();
  if (s.includes('.')) return s;
  if (exchange.toUpperCase() === 'NSE') return `${s}.NS`;
  if (exchange.toUpperCase() === 'BSE') return `${s}.BO`;
  throw new Error('unsupported exchange');
}

function classifyFreshness(baseline, ageSeconds, staleAfter = 300, offlineAfter = 21600) {
  if (ageSeconds >= offlineAfter) return 'offline';
  if (ageSeconds >= staleAfter) return 'stale';
  return baseline;
}

function backoff(failures, base = 5, max = 300) {
  if (failures <= 0) return base;
  return Math.min(max, base * 2 ** Math.min(failures - 1, 10));
}

function estimateNseSession(epochMs) {
  const ist = new Date(epochMs + 330 * 60 * 1000);
  const day = ist.getUTCDay();
  if (day === 0 || day === 6) return 'closed';
  const minutes = ist.getUTCHours() * 60 + ist.getUTCMinutes();
  return minutes >= 9 * 60 + 15 && minutes <= 15 * 60 + 30 ? 'open' : 'closed';
}

function dedupeTargets(holdings, watchlist) {
  const map = new Map();
  for (const row of [...holdings, ...watchlist]) {
    const exchange = row.exchange.trim().toUpperCase();
    const symbol = row.symbol.trim().toUpperCase();
    map.set(`${exchange}:${symbol}`, { exchange, symbol });
  }
  return [...map.values()].sort((a,b)=>`${a.exchange}:${a.symbol}`.localeCompare(`${b.exchange}:${b.symbol}`));
}

function mergePartialCache(cache, received) {
  const result = new Map(cache.map(q => [`${q.exchange}:${q.symbol}`, q]));
  for (const quote of received) result.set(`${quote.exchange}:${quote.symbol}`, quote);
  return [...result.values()];
}

test('Yahoo delayed adapter maps NSE and BSE symbols without touching explicit suffixes', () => {
  assert.equal(providerSymbol('NSE','DEMOTECH'), 'DEMOTECH.NS');
  assert.equal(providerSymbol('BSE','500300'), '500300.BO');
  assert.equal(providerSymbol('NSE','DEMOPHARMA.NS'), 'DEMOPHARMA.NS');
});

test('explicit provider symbol overrides take precedence conceptually', () => {
  const instrument = {exchange:'NSE', symbol:'DEMOBANK', providerSymbol:'DEMOBANK.BO'};
  const mapped = instrument.providerSymbol ?? providerSymbol(instrument.exchange, instrument.symbol);
  assert.equal(mapped, 'DEMOBANK.BO');
});

test('delayed quote ages to stale then offline and is never promoted to live', () => {
  assert.equal(classifyFreshness('delayed', 5), 'delayed');
  assert.equal(classifyFreshness('delayed', 301), 'stale');
  assert.equal(classifyFreshness('delayed', 21600), 'offline');
});

test('backoff doubles deterministically and respects cap', () => {
  assert.equal(backoff(1,5,60),5);
  assert.equal(backoff(2,5,60),10);
  assert.equal(backoff(3,5,60),20);
  assert.equal(backoff(10,5,60),60);
});

test('normal NSE weekday clock estimate opens at 09:15 IST and closes after 15:30', () => {
  const monday = Date.UTC(2026, 10, 9, 3, 45); // 09:15 IST Monday
  assert.equal(estimateNseSession(monday), 'open');
  assert.equal(estimateNseSession(Date.UTC(2026,10,9,10,0)), 'open'); // 15:30 IST
  assert.equal(estimateNseSession(Date.UTC(2026,10,9,10,1)), 'closed');
  assert.equal(estimateNseSession(Date.UTC(2026,10,8,6,0)), 'closed'); // Sunday
});

test('holdings and watchlist quote targets are normalized and deduplicated', () => {
  const rows = dedupeTargets(
    [{exchange:'nse',symbol:' demotech '},{exchange:'NSE',symbol:'DEMOETF'}],
    [{exchange:'NSE',symbol:'DEMOTECH'},{exchange:'NSE',symbol:'DEMOPHARMA'}],
  );
  assert.deepEqual(rows, [
    {exchange:'NSE',symbol:'DEMOETF'},
    {exchange:'NSE',symbol:'DEMOPHARMA'},
    {exchange:'NSE',symbol:'DEMOTECH'},
  ]);
});

test('partial provider response updates returned symbols without deleting cached failures', () => {
  const cache = [
    {exchange:'NSE',symbol:'DEMOTECH',price:290,source:'old'},
    {exchange:'NSE',symbol:'DEMOETF',price:116,source:'old'},
  ];
  const merged = mergePartialCache(cache,[{exchange:'NSE',symbol:'DEMOTECH',price:300,source:'new'}]);
  assert.equal(merged.find(q=>q.symbol==='DEMOTECH').price,300);
  assert.equal(merged.find(q=>q.symbol==='DEMOETF').price,116);
});

test('provider provenance survives cache merge', () => {
  const quote = {exchange:'NSE',symbol:'DEMOTECH',price:299.3,source:'yahoo_delayed',observedUnixMs:123};
  const merged = mergePartialCache([], [quote]);
  assert.deepEqual(merged[0], quote);
});

function automaticProvider(tokenPresent) {
  return tokenPresent ? 'upstox_yahoo_fallback' : 'yahoo_delayed';
}

test('automatic provider prefers official Upstox only when a token is configured', () => {
  assert.equal(automaticProvider(true), 'upstox_yahoo_fallback');
  assert.equal(automaticProvider(false), 'yahoo_delayed');
});

test('official REST snapshots are fresh while Yahoo fallback remains delayed', () => {
  assert.equal(classifyFreshness('fresh', 5), 'fresh');
  assert.equal(classifyFreshness('delayed', 5), 'delayed');
  assert.equal(classifyFreshness('fresh', 301), 'stale');
});

test('mixed refresh provenance is preserved per instrument', () => {
  const cache = [];
  const received = [
    {exchange:'NSE',symbol:'AAA',price:100,source:'upstox_analytics'},
    {exchange:'NSE',symbol:'BBB',price:200,source:'yahoo_delayed'},
  ];
  const merged = mergePartialCache(cache, received);
  assert.equal(merged.find(q=>q.symbol==='AAA').source, 'upstox_analytics');
  assert.equal(merged.find(q=>q.symbol==='BBB').source, 'yahoo_delayed');
});
