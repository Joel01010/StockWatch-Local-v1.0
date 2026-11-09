# Phase 7 reliability and hardening

StockWatch Local keeps its always-on footprint small by putting maintenance on a low-frequency Rust worker rather than adding another process or WebView.

## Database durability

- SQLite runs with foreign keys, a 5-second busy timeout, WAL journaling, `synchronous=NORMAL`, and automatic WAL checkpointing.
- All schema migrations are applied inside one SQLite transaction and the resulting schema version is verified.
- Before an older database is migrated, StockWatch creates and verifies a `pre_migration` backup.
- `PRAGMA quick_check` is mandatory on startup recovery paths and on maintenance runs.

## Backups and restore

Backups use SQLite `VACUUM INTO`, not a raw copy of an open database. Every generated backup is opened read-only and must pass `quick_check` before it is accepted.

Restore never overwrites the running database. A selected verified backup is copied to `stockwatch.restore.pending.db`, synced, and verified. On the next process start, before SQLite is opened, the current database is preserved as `stockwatch.pre_restore.<timestamp>.db` and the pending image is installed. A failed post-copy verification rolls the operation back.

If the primary database itself fails startup `quick_check`, StockWatch preserves it as `stockwatch.corrupt.<timestamp>.db` and can recover from the newest verified backup. If no verified backup exists, startup fails rather than silently creating a blank portfolio.

## Bounded growth

Phase 7 adds configurable retention for quote history, performance snapshots, notification history, automatic backups, and rotating event logs. Maintenance defaults to once per hour and starts after a short delay so it is not on the startup hot path.

Performance snapshots remain five-minute buckets from Phase 6. Retention deletes old parent snapshots and relies on the existing foreign-key cascade to remove their holding rows.

## Provider and wake recovery

Provider failures continue to keep the last known cache and age its freshness. A wall-clock gap detector identifies scheduler gaps consistent with sleep/hibernate. On such a gap the failure backoff is reset, cached freshness is recomputed immediately, and the next eligible market refresh is not delayed by an old failure backoff.

This is software behavior only. Actual Windows sleep/hibernate, network transitions and timer behavior are qualified in Phase 8.

## Diagnostics

The Settings page exposes database health, schema version, database size, verified backups, refresh success/error counts, last refresh duration and scheduler-gap recovery count. These are application-level metrics; Phase 7 does not claim operating-system RAM/CPU/handle measurements.

## Event logging

The local event log records maintenance/startup recovery and provider failures. It is size-capped and rotated to a bounded number of files. No cloud logging or telemetry is introduced.
