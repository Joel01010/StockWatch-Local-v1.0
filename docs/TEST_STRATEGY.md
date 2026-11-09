# Test strategy

## Phases 0–7: software/automated checks only

Allowed: unit tests, integration tests, SQLite tests, migration checks, static validation, JSON/TOML validation, frontend build/type checks when toolchains are available, mocked provider tests, deterministic scheduler/alert simulations, dependency audits and source-level architecture checks.

Not claimed before Phase 8: actual Windows tray interaction, toast delivery, startup registration, WebView destruction/recreation behavior under Windows, Windows sleep/resume, installer behavior, taskbar positioning, clean uninstall, or real Windows RAM/CPU measurements.

Status language:
- PASS — automated
- PASS — static/build check
- SIMULATED PASS
- IMPLEMENTED — Windows verification deferred
- NOT RUN — missing toolchain/dependency
- SKIPPED — requires Windows qualification

## Phase 8: Windows qualification
All native behavior and real resource measurements are executed on a real Windows environment.
