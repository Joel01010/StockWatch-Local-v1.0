param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path $PSScriptRoot 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-ALL'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$dir=Join-Path $OutputDir 'diagnostics';New-Item -ItemType Directory -Force -Path $dir|Out-Null
$scripts=Get-ChildItem (Join-Path $PSScriptRoot 'diagnostics') -Filter 'DIAG_*.ps1' | Where-Object {$_.Name -ne 'DIAG_SLEEP_RESUME.ps1'} | Sort-Object Name
foreach($s in $scripts){Write-Host "=== $($s.Name) ===" -ForegroundColor Cyan;try{& $s.FullName -OutputDir $dir}catch{Write-Warning $_.Exception.Message}}
$bundle=& (Join-Path $PSScriptRoot 'COLLECT_SUPPORT_BUNDLE.ps1') -SessionDir $OutputDir
Write-Host "RETURN_THIS_FILE=$bundle" -ForegroundColor Yellow
