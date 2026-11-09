import test from 'node:test';
import assert from 'node:assert/strict';

function derivePortfolio(trades) {
  const ordered = [...trades].sort((a, b) =>
    Date.parse(a.executedAt) - Date.parse(b.executedAt) || (a.id ?? Number.MAX_SAFE_INTEGER) - (b.id ?? Number.MAX_SAFE_INTEGER)
  );
  const positions = new Map();
  let realizedGrossPnl = 0;
  let feesPaid = 0;

  for (const [index, trade] of ordered.entries()) {
    if (!trade.exchange || !trade.symbol) throw new Error(`invalid instrument ${index}`);
    if (!(trade.quantity > 0)) throw new Error(`invalid quantity ${index}`);
    if (trade.price < 0) throw new Error(`invalid price ${index}`);
    if (trade.fees < 0) throw new Error(`invalid fees ${index}`);

    const key = `${trade.exchange.toUpperCase()}:${trade.symbol.toUpperCase()}`;
    const p = positions.get(key) ?? {
      exchange: trade.exchange.toUpperCase(), symbol: trade.symbol.toUpperCase(),
      quantity: 0, averagePrice: 0, realizedGrossPnl: 0, feesPaid: 0,
    };
    feesPaid += trade.fees;
    p.feesPaid += trade.fees;

    if (trade.side === 'buy') {
      const previousCost = p.quantity * p.averagePrice;
      const newQuantity = p.quantity + trade.quantity;
      p.averagePrice = (previousCost + trade.quantity * trade.price) / newQuantity;
      p.quantity = newQuantity;
    } else if (trade.side === 'sell') {
      if (trade.quantity > p.quantity) throw new Error(`oversell ${key}`);
      const realized = trade.quantity * (trade.price - p.averagePrice);
      p.realizedGrossPnl += realized;
      realizedGrossPnl += realized;
      p.quantity -= trade.quantity;
      if (p.quantity === 0) p.averagePrice = 0;
    } else {
      throw new Error('invalid side');
    }
    positions.set(key, p);
  }
  return { positions: [...positions.values()], realizedGrossPnl, feesPaid };
}

const trade = (side, quantity, price, fees, executedAt, symbol='TEST') => ({
  exchange: 'NSE', symbol, side, quantity, price, fees, executedAt,
});

test('weighted average uses buys only', () => {
  const r = derivePortfolio([
    trade('buy', 2, 100, 1, '2026-01-01T10:00:00Z'),
    trade('buy', 1, 130, 1, '2026-01-02T10:00:00Z'),
  ]);
  assert.equal(r.positions[0].quantity, 3);
  assert.equal(r.positions[0].averagePrice, 110);
  assert.equal(r.feesPaid, 2);
});

test('partial sell realizes against weighted average and preserves average', () => {
  const r = derivePortfolio([
    trade('buy', 2, 100, 1, '2026-01-01T10:00:00Z'),
    trade('buy', 2, 120, 1, '2026-01-02T10:00:00Z'),
    trade('sell', 1, 140, 2, '2026-01-03T10:00:00Z'),
  ]);
  assert.equal(r.positions[0].quantity, 3);
  assert.equal(r.positions[0].averagePrice, 110);
  assert.equal(r.realizedGrossPnl, 30);
  assert.equal(r.realizedGrossPnl - r.feesPaid, 26);
});

test('full close resets average before a later re-entry', () => {
  const r = derivePortfolio([
    trade('buy', 1, 100, 0, '2026-01-01T10:00:00Z'),
    trade('sell', 1, 120, 0, '2026-01-02T10:00:00Z'),
    trade('buy', 1, 75, 0, '2026-01-03T10:00:00Z'),
  ]);
  assert.equal(r.positions[0].quantity, 1);
  assert.equal(r.positions[0].averagePrice, 75);
  assert.equal(r.realizedGrossPnl, 20);
});

test('oversells are rejected', () => {
  assert.throws(() => derivePortfolio([
    trade('buy', 1, 100, 0, '2026-01-01T10:00:00Z'),
    trade('sell', 2, 120, 0, '2026-01-02T10:00:00Z'),
  ]), /oversell/);
});

test('timestamp ordering makes reversed input deterministic', () => {
  const r = derivePortfolio([
    trade('sell', 1, 110, 0, '2026-01-02T10:00:00Z'),
    trade('buy', 1, 100, 0, '2026-01-01T10:00:00Z'),
  ]);
  assert.equal(r.realizedGrossPnl, 10);
});

test('multiple symbols stay isolated', () => {
  const r = derivePortfolio([
    trade('buy', 2, 100, 0, '2026-01-01T10:00:00Z', 'AAA'),
    trade('buy', 3, 10, 0, '2026-01-01T10:00:00Z', 'BBB'),
    trade('sell', 1, 120, 0, '2026-01-02T10:00:00Z', 'AAA'),
  ]);
  const a = r.positions.find(p => p.symbol === 'AAA');
  const b = r.positions.find(p => p.symbol === 'BBB');
  assert.equal(a.quantity, 1);
  assert.equal(a.realizedGrossPnl, 20);
  assert.equal(b.quantity, 3);
  assert.equal(b.averagePrice, 10);
});

test('exact invested amount avoids rounded-average drift in synthetic data', () => {
  const rows = [
    { quantity: 3, average: 100.00, invested: 300.00 },
    { quantity: 4, average: 200.00, invested: 800.01 },
    { quantity: 6, average: 150.00, invested: 900.00 },
    { quantity: 2, average: 250.00, invested: 500.00 },
  ];
  const naive = rows.reduce((sum, r) => sum + r.quantity * r.average, 0);
  const exact = rows.reduce((sum, r) => sum + r.invested, 0);
  assert.equal(Number(naive.toFixed(2)), 2500.00);
  assert.equal(Number(exact.toFixed(2)), 2500.01);
});


test('RFC3339 offsets are ordered by actual instant', () => {
  const r = derivePortfolio([
    trade('sell', 1, 110, 0, '2026-01-01T05:00:01Z'),
    trade('buy', 1, 100, 0, '2026-01-01T10:00:00+05:30'),
  ]);
  assert.equal(r.realizedGrossPnl, 10);
});
