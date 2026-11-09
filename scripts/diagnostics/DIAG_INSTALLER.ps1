param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-INSTALLER'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'installer.txt';"=== INSTALLER ==="|Set-Content $out -Encoding UTF8
$installer=Get-LatestNsisInstaller
if($installer){"INSTALLER=$installer"|Add-Content $out;Get-Item $installer|Select-Object Name,Length,CreationTime,LastWriteTime,VersionInfo|Format-List|Out-String|Add-Content $out;Get-FileHash $installer -Algorithm SHA256|Format-List|Out-String|Add-Content $out;Get-AuthenticodeSignature $installer|Format-List Status,StatusMessage,Path|Out-String|Add-Content $out}else{"NO_NSIS_INSTALLER"|Add-Content $out}
"--- Installed entries ---"|Add-Content $out
try{Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue|Where-Object{$_.DisplayName -like 'StockWatch Local*'}|Select-Object DisplayName,DisplayVersion,InstallLocation,UninstallString,QuietUninstallString|Format-List|Out-String|Add-Content $out}catch{}
Get-Content $out
