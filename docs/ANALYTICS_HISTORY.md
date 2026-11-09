# Analytics & History architecture

## Scope

Phase 6 adds deterministic portfolio analytics without AI and without inventing broker cash movements.

The analytics subsystem is deliberately **securities-only**: portfolio market value is the value of tracked open holdings. StockWatch does not assume that a stock purchase equals a brokerage-account deposit and does not treat sale proceeds as an external cash flow.

## Cash-flow ledger

Users may explicitly record four local cash-flow kinds:

- `deposit` — external capital entering the tracked portfolio;
- `withdrawal` — external capital leaving the tracked portfolio;
- `dividend` — investment income;
- `interest` — investment income.

Only deposits and withdrawals are external flows for performance-return calculations. Dividends and interest are counted as investment income and contribute to tracked P&L.

## Performance snapshots

A successful market refresh attempts to create a performance snapshot only when **every open holding has a usable `live`, `fresh`, or explicitly `delayed` price**. Missing, stale, or offline holdings cause the snapshot to be skipped.

Snapshots are grouped into five-minute buckets. Repeated successful refreshes inside a bucket replace the bucket rather than append unbounded rows. Each snapshot records:

- portfolio securities value;
- open cost basis;
- realized gross P&L;
- recorded transaction fees;
- cumulative investment income;
- cumulative net external flow;
- provider/source.

Per-holding snapshot rows preserve quantity, price, value, and open cost basis for later allocation-history work.

## Cash-flow-adjusted return

StockWatch uses **Modified Dietz** over the available snapshot period. For a period beginning at `BMV` and ending at `EMV` with explicit external flows `CF_i`:

`R = (EMV - BMV - ΣCF_i) / (BMV + Σ(w_i × CF_i))`

where `w_i` is the fraction of the period remaining after each flow. A deposit is positive and a withdrawal is negative.

This prevents capital additions from appearing as investment gains. If there are fewer than two valid snapshots, or the denominator is zero, the return is reported as unavailable rather than guessed.

## P&L composition

Tracked net P&L is:

`unrealized + realized gross + dividends + interest - recorded transaction fees`

Only fees actually present in the transaction ledger are included. StockWatch does not fabricate missing brokerage/statutory charges.

## Benchmark support

Analytics may optionally store one benchmark exchange/symbol. When configured it joins the normal quote target union; no extra background process is created. Once overlapping quote history exists, StockWatch compares the benchmark's simple price return against the portfolio's cash-flow-adjusted return.

The benchmark is optional. Absence of history is surfaced as unavailable, not as zero return.

## Persistence

Schema v5 adds:

- `cash_flows`;
- `performance_snapshots`;
- `performance_holding_snapshots`;
- `analytics_preferences`.

All data remains in the local SQLite database.

## Resource behavior

Phase 6 does not add a new daemon, runtime, WebView, polling loop, or AI component. Snapshot persistence is piggybacked on successful market refreshes and compacted to one row per five-minute bucket.

## Qualification status

This phase is verified with software-level reference, schema, structural, and regression checks only. Real Windows resource measurements and native runtime qualification remain deferred to Phase 8.
