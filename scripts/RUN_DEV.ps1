$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not (Get-Command node -ErrorAction SilentlyContinue)) { throw "Node.js is required." }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw "Rust/Cargo is required." }
if (-not (Test-Path node_modules)) { npm install }
npm run tauri dev
