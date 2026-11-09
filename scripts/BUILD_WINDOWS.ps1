. (Join-Path $PSScriptRoot 'lib\TestHarness.Common.ps1')
$ErrorActionPreference = "Stop"
[void](Initialize-StockWatchVisualStudioEnvironment)
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not $IsWindows) { throw "This script is for Windows." }
if (-not (Get-Command node -ErrorAction SilentlyContinue)) { throw "Node.js is required." }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw "Rust/Cargo is required." }
if (-not (Test-Path node_modules)) { npm install --no-audit --no-fund }

.\scripts\CHECK_SOFTWARE_ONLY.ps1
npm run check:phase8
npm run test:release
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run build
npm run tauri build -- --bundles nsis
