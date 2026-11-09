# CSV import formats — Phase 1

## Transaction import
Required header:

```csv
exchange,symbol,name,side,quantity,price,fees,executed_at,external_ref
```

`name`, `fees`, and `external_ref` may be blank. `side` accepts `buy`, `b`, `sell`, or `s`. `executed_at` must be RFC3339.

Synthetic example:

```csv
NSE,DEMOTECH,Demo Technology,buy,4,200.00,0.00,2026-11-01T10:00:00Z,demotech-b1
```

A non-null `external_ref` is the idempotency key. Importing the same row again does not create a second transaction.

## Opening-holdings snapshot
Header:

```csv
exchange,symbol,name,quantity,average_price,invested_amount
```

`name` and `invested_amount` may be blank. If `invested_amount` is present, StockWatch reconstructs the internal opening price from `invested_amount / quantity`; this preserves exact broker cost basis when the displayed average is rounded.

The provided `examples/current-holdings.csv` uses synthetic data to demonstrate the rounding case: a displayed average of ₹200.00 for quantity 4 multiplies to ₹800.00, while an exact imported cost basis of ₹800.01 preserves the one-paise difference.

Opening holdings are converted into deterministic synthetic buy transactions whose external reference contains the caller-supplied import batch id.
