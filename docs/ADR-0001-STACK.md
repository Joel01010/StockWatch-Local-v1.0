# ADR-0001 — Native Rust core with disposable Svelte UI

Status: Accepted

## Decision
Use Rust + Tauri 2 + Svelte/TypeScript + SQLite. Do not use Electron, Python/FastAPI, Node background services, Docker, PostgreSQL, Redis, or AI components.

## Rationale
The product is expected to remain resident for long periods while doing very little work. A native Rust core avoids a permanent browser/runtime backend. Tauri provides Windows shell integration while permitting a lightweight WebView dashboard only when required.

## Consequences
- More Rust implementation work than an all-JavaScript desktop app.
- Windows-native behavior must be qualified on Windows only at the final gate.
- Domain logic must remain separable from native adapters to maximize automated test coverage off-Windows.
