param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-SECURITY'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'security.txt';"=== WINDOWS SECURITY ==="|Set-Content $out -Encoding UTF8
try{Get-MpComputerStatus|Select-Object AntivirusEnabled,RealTimeProtectionEnabled,IoavProtectionEnabled,BehaviorMonitorEnabled,AntispywareEnabled,NISEnabled|Format-List|Out-String|Add-Content $out}catch{"DEFENDER_STATUS_UNAVAILABLE=$($_.Exception.Message)"|Add-Content $out}
try{Get-MpPreference|Select-Object EnableControlledFolderAccess,PUAProtection,DisableRealtimeMonitoring|Format-List|Out-String|Add-Content $out}catch{}
try{Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-Windows Defender/Operational';StartTime=(Get-Date).AddHours(-4)} -ErrorAction Stop|Where-Object{$_.Message -match 'stockwatch|blocked|quarantine'}|Select-Object -First 50 TimeCreated,Id,Message|Format-List|Out-String|Add-Content $out}catch{}
try{Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-CodeIntegrity/Operational';StartTime=(Get-Date).AddHours(-4)} -ErrorAction Stop|Where-Object{$_.Message -match 'stockwatch'}|Select-Object -First 50 TimeCreated,Id,Message|Format-List|Out-String|Add-Content $out}catch{}
$exe=Get-StockWatchReleaseExe;if($exe){Get-AuthenticodeSignature $exe|Format-List Status,StatusMessage,Path,SignerCertificate|Out-String|Add-Content $out}
Get-Content $out
