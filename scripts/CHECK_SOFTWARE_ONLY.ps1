$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)
. (Join-Path $PSScriptRoot 'lib\TestHarness.Common.ps1')
[void](Initialize-StockWatchVisualStudioEnvironment)

npm run check:phase0
npm run check:phase1
npm run check:phase2
npm run check:phase3
npm run check:phase4
npm run check:phase5
npm run check:phase6
npm run check:phase7
npm run check:phase8
if ((Get-Content package.json -Raw) -match 'check:harness') { npm run check:harness }
npm run check:tscore
npm run check:tsbridge
npm test
npm run test:release

$python = $null
$pythonArgs = @()
if (Get-Command python -ErrorAction SilentlyContinue) { $python='python' }
elseif (Get-Command python3 -ErrorAction SilentlyContinue) { $python='python3' }
elseif (Get-Command py -ErrorAction SilentlyContinue) { $python='py'; $pythonArgs=@('-3') }
else { throw "Python 3 is required for the dependency-free SQLite checks." }
foreach($script in @('phase1-db-check.py','phase4-db-check.py','phase5-db-check.py','phase6-db-check.py','phase7-db-check.py','phase7-soak.py')) {
  & $python @pythonArgs (Join-Path 'tools' $script)
  if($LASTEXITCODE -ne 0){throw "$script failed with exit code $LASTEXITCODE"}
}

if (Test-Path node_modules) {
  npm run build
} else {
  Write-Warning "node_modules not found: Svelte/Vite production build was NOT RUN."
}

if (Get-Command cargo -ErrorAction SilentlyContinue) {
  cargo test --manifest-path src-tauri/Cargo.toml
  cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
  cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
} else {
  Write-Warning "Rust/Cargo not found: Rust compile/test/fmt/clippy checks were NOT RUN."
}

Write-Host "PASS: available Phase 0-8 software checks completed."
Write-Host "Native Windows qualification requires scripts/QUALIFY_WINDOWS.ps1 on Windows."
