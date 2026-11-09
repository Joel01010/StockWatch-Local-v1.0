param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-NOTIFY'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'notifications.txt';"=== NOTIFICATIONS ==="|Set-Content $out -Encoding UTF8
$installed=$null
try{$entries=Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue|Where-Object{$_.DisplayName -like 'StockWatch Local*'};$installed=$entries|Select-Object -First 1;if($installed){"INSTALLED_ENTRY=$($installed.DisplayName) location=$($installed.InstallLocation)"|Add-Content $out}else{"NO_INSTALLED_ENTRY"|Add-Content $out}}catch{}
"NOTE: authoritative Windows toast test is performed by QUALIFY_INSTALLED_WINDOWS.ps1 after NSIS install."|Add-Content $out
try{Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\*' -ErrorAction SilentlyContinue|Where-Object{$_.PSChildName -match 'StockWatch|stockwatch'}|Format-List|Out-String|Add-Content $out}catch{}
Get-Content $out
