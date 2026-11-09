param([string]$OutputDir='',[switch]$AllowSuspend)
$ErrorActionPreference='Stop'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-SLEEP'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'sleep-resume.txt';"=== SLEEP / RESUME ==="|Set-Content $out -Encoding UTF8
if(-not $AllowSuspend){"NOT_RUN: This script can suspend the computer. Re-run only with -AllowSuspend when explicitly authorized."|Add-Content $out;Get-Content $out;return}
$exe=Get-StockWatchReleaseExe;if(-not $exe){"NO_EXE"|Add-Content $out;Get-Content $out;throw 'StockWatch executable not found.'}
$p=Start-Process $exe -PassThru;"BEFORE_SUSPEND pid=$($p.Id) time=$((Get-Date).ToString('o'))"|Add-Content $out
Start-Sleep 3
rundll32.exe powrprof.dll,SetSuspendState 0,1,0
Start-Sleep 15
$p.Refresh();"AFTER_RESUME exited=$($p.HasExited) time=$((Get-Date).ToString('o'))"|Add-Content $out
if(-not $p.HasExited){Stop-Process $p.Id -Force}
Get-Content $out
