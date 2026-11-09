# Portfolio core — Phase 1

## Cost basis model
StockWatch Local uses a weighted-average acquisition price for an open position:

`new_average = (old_quantity * old_average + buy_quantity * buy_price) / new_quantity`

A sell does not alter the average price of the remaining open quantity. When the position reaches exactly zero, the average price is reset to zero so a later re-entry starts with a fresh cost basis.

## P&L model
For a sell:

`realized_gross = sold_quantity * (sell_price - current_average_price)`

Transaction fees are tracked separately from the displayed acquisition average. The Phase 1 helper `net_realized_after_all_fees` is intentionally a simple personal-performance measure rather than an income-tax computation.

Tax-lot accounting, statutory charge schedules, corporate actions and tax reporting are later concerns and must not be inferred from this Phase 1 metric.

## Exact decimals
Core calculations use `rust_decimal::Decimal`; they do not use binary floating-point arithmetic. The inherited Phase 0 SQLite schema stores transaction numeric fields with SQLite REAL affinity, so the repository converts values at the storage boundary and normalizes them to 12 decimal places when loading. This is acceptable for the Phase 1 personal-stock use case but remains an explicit storage limitation; a future migration can move canonical amounts to decimal text/integer minor units if broader asset classes require it.

## Atomicity
`insert_trades_atomic` opens a SQLite transaction, inserts all non-duplicate rows, reloads the resulting complete ledger, derives the portfolio, and commits only if derivation succeeds. Oversells therefore roll back the import batch.

## Determinism
Trades are ordered by RFC3339 execution timestamp, then persisted row id, then original input order. Imports should normalize timestamps to a common offset/UTC when possible.
