param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-WEBVIEW2'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'webview2.txt';"=== WEBVIEW2 ==="|Set-Content $out -Encoding UTF8
$keys=@(
'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F1E7E5A5-4A3B-4B54-9B08-1B6D6A7C928B}',
'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F1E7E5A5-4A3B-4B54-9B08-1B6D6A7C928B}',
'HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F1E7E5A5-4A3B-4B54-9B08-1B6D6A7C928B}'
)
foreach($k in $keys){if(Test-Path $k){"FOUND $k"|Add-Content $out;Get-ItemProperty $k|Select-Object name,pv,location|Format-List|Out-String|Add-Content $out}else{"NOT_FOUND $k"|Add-Content $out}}
$paths=@("$env:ProgramFiles(x86)\Microsoft\EdgeWebView\Application","$env:ProgramFiles\Microsoft\EdgeWebView\Application","$env:LOCALAPPDATA\Microsoft\EdgeWebView\Application")
foreach($p in $paths){if(Test-Path $p){"DIR $p"|Add-Content $out;Get-ChildItem $p -Directory -ErrorAction SilentlyContinue|Select-Object Name,FullName|Format-Table|Out-String|Add-Content $out}}
Get-Process msedgewebview2 -ErrorAction SilentlyContinue|Select-Object Id,Path,WorkingSet64,PrivateMemorySize64|Format-Table|Out-String|Add-Content $out
Get-Content $out
