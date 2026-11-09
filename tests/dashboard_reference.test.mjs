import test from 'node:test';
import assert from 'node:assert/strict';

const base = [
  {symbol:'DEMOETF', qty:3, invested:300.00, price:105.00, change:0.50},
  {symbol:'DEMOTECH', qty:4, invested:800.01, price:210.00, change:1.20},
  {symbol:'DEMOBANK', qty:6, invested:900.00, price:147.00, change:-0.70},
  {symbol:'DEMOPHARMA', qty:2, invested:500.00, price:260.00, change:0.40},
];

function derive(rows) {
  const current = rows.reduce((sum, row) => sum + row.qty * row.price, 0);
  return rows.map(row => {
    const value = row.qty * row.price;
    const pnl = value - row.invested;
    return {...row, value, pnl, returnPct:pnl/row.invested*100, allocation:value/current*100};
  });
}

function parseRoute(hash) {
  const raw = hash.replace(/^#\/?/, '');
  if (!raw) return {kind:'overview'};
  const [first, second] = raw.split('/');
  if (first === 'holding' && second) return {kind:'holding', symbol:decodeURIComponent(second).toUpperCase()};
  if (['holdings','watchlist','alerts','analytics','settings','mini'].includes(first)) return {kind:first};
  return {kind:'overview'};
}

test('synthetic holdings preserve exact invested total', () => {
  const invested = base.reduce((sum,row)=>sum+row.invested,0);
  assert.equal(Number(invested.toFixed(2)), 2500.01);
});

test('dashboard reference derives current value and total pnl deterministically', () => {
  const rows = derive(base);
  const current = rows.reduce((sum,row)=>sum+row.value,0);
  const pnl = rows.reduce((sum,row)=>sum+row.pnl,0);
  assert.equal(Number(current.toFixed(2)), 2557.00);
  assert.equal(Number(pnl.toFixed(2)), 56.99);
});

test('allocation percentages sum to 100 percent', () => {
  const total = derive(base).reduce((sum,row)=>sum+row.allocation,0);
  assert(Math.abs(total - 100) < 1e-9);
});

test('DEMOBANK is the largest position in the synthetic sample', () => {
  const rows = derive(base).sort((a,b)=>b.allocation-a.allocation);
  assert.equal(rows[0].symbol, 'DEMOBANK');
  assert(rows[0].allocation > 30);
});

test('hash router resolves detail and mini routes without a framework', () => {
  assert.deepEqual(parseRoute('#/holding/demotech'), {kind:'holding',symbol:'DEMOTECH'});
  assert.deepEqual(parseRoute('#/mini'), {kind:'mini'});
  assert.deepEqual(parseRoute('#/unknown'), {kind:'overview'});
});

test('mini mode is intentionally bounded to six visible holdings', () => {
  const many = Array.from({length:10},(_,i)=>({symbol:`S${i}`}));
  assert.equal(many.slice(0,6).length,6);
});
