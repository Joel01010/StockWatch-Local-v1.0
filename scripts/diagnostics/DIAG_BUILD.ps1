param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
$Root=Get-StockWatchRoot;Set-Location $Root
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-BUILD'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'build.txt';"=== BUILD DIAGNOSTIC ==="|Set-Content $out -Encoding UTF8
function Run([string]$label,[scriptblock]$b){"--- $label ---"|Add-Content $out;try{& $b *>&1|Add-Content $out;"EXIT=$LASTEXITCODE"|Add-Content $out}catch{"ERROR=$($_.Exception.Message)"|Add-Content $out}}
Run 'npm-version' {npm --version}
Run 'npm-registry' {npm config get registry}
Run 'npm-cache' {npm config get cache}
Run 'npm-package' {Get-Content package.json -Raw}
Run 'cargo-version' {cargo --version}
Run 'cargo-metadata' {cargo metadata --manifest-path src-tauri/Cargo.toml --no-deps --format-version 1}
Run 'cargo-check' {cargo check --manifest-path src-tauri/Cargo.toml --all-targets}
if(Test-Path node_modules){Run 'frontend-build' {npm run build}}else{"SKIP frontend-build: node_modules missing"|Add-Content $out}
Get-Content $out
