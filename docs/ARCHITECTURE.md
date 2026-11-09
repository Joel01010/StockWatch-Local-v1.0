# StockWatch Local architecture — Phase 1

## Product boundary
StockWatch Local is a read-only, local-first Windows desktop market monitor. It does not place orders, make investment recommendations, run AI models, or require a cloud backend.

## Locked stack
- Rust: long-lived background core, calculations, persistence, polling, alerts, tray state.
- Tauri 2: Windows desktop shell and native tray integration.
- Svelte + TypeScript: disposable dashboard WebView only.
- SQLite: local transaction, settings, quote, alert, and history storage.
- reqwest + rustls: provider HTTP transport.
- tokio: async scheduling and provider work.

## Process model
The intended release architecture is one native process. The UI is not authoritative: portfolio state, alert state, provider state and scheduling live in Rust. Phase 2 will make the tray the primary interface and allow the main WebView to be closed/recreated.

## Phase 1 portfolio boundary
`portfolio` is a platform-neutral domain module. It has no Tauri, Windows, HTTP, or UI dependencies. `imports` parses external rows into domain trades. `storage` persists/reloads those trades and validates the complete ledger before committing an import batch.

Dependency direction:

UI -> Tauri commands/events -> application/core modules -> domain + SQLite/provider adapters.

## Safety/correctness invariants
1. Never present cached data as live.
2. Every quote carries source and observation time/freshness.
3. Broker/order execution is out of scope for v1.0.
4. Secrets must not be persisted in ordinary SQLite settings.
5. Transaction history is the long-term source of truth for cost basis.
6. Provider failure must degrade to explicit stale/offline state.
7. Alert rules are deterministic and unit-testable.
8. No Windows-only verification is claimed before final qualification.
9. A sell may not reduce a holding below zero.
10. Import writes are atomic: a ledger-invalid batch must roll back.
11. Re-imported rows with the same non-null external reference are idempotently skipped.
12. Broker-reported exact invested amounts may override rounded displayed averages during opening-holdings import.
