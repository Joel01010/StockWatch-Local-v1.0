param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-AUTOSTART'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'autostart.txt';"=== AUTOSTART ==="|Set-Content $out -Encoding UTF8
try{$r=Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run';$r.PSObject.Properties|Where-Object{$_.Name -match 'StockWatch|stockwatch'}|Format-Table Name,Value|Out-String|Add-Content $out}catch{}
$exe=Get-StockWatchReleaseExe
if($exe){
 $initial=$null
 $statusReport=Join-Path $OutputDir 'autostart-status.report'
 try{$p=Start-StockWatchIsolatedProcess -Exe $exe -BaseDir $OutputDir -ExtraEnvironment @{STOCKWATCH_QUALIFICATION_AUTOSTART='status';STOCKWATCH_QUALIFICATION_REPORT=$statusReport;STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS='1200'};if(-not $p.WaitForExit(10000)){Stop-Process $p.Id -Force};$st=if(Test-Path $statusReport){Get-Content $statusReport -Raw}else{''};$initial=($st -match 'autostart_enabled=true');"INITIAL $st"|Add-Content $out}catch{"STATUS_ERROR=$($_.Exception.Message)"|Add-Content $out}
 try{
  foreach($action in @('enable','disable')){
   $report=Join-Path $OutputDir "autostart-$action.report";$p=Start-StockWatchIsolatedProcess -Exe $exe -BaseDir $OutputDir -ExtraEnvironment @{STOCKWATCH_QUALIFICATION_AUTOSTART=$action;STOCKWATCH_QUALIFICATION_REPORT=$report;STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS='1200'};if(-not $p.WaitForExit(10000)){Stop-Process $p.Id -Force};if(Test-Path $report){Get-Content $report|Add-Content $out}else{"NO_REPORT $action"|Add-Content $out}
  }
 }catch{"ERROR=$($_.Exception.Message)"|Add-Content $out}
 finally{
  if($initial -eq $true){$p=Start-StockWatchIsolatedProcess -Exe $exe -BaseDir $OutputDir -ExtraEnvironment @{STOCKWATCH_QUALIFICATION_AUTOSTART='enable';STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS='1200'};if(-not $p.WaitForExit(10000)){Stop-Process $p.Id -Force}}
  elseif($initial -eq $false){$p=Start-StockWatchIsolatedProcess -Exe $exe -BaseDir $OutputDir -ExtraEnvironment @{STOCKWATCH_QUALIFICATION_AUTOSTART='disable';STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS='1200'};if(-not $p.WaitForExit(10000)){Stop-Process $p.Id -Force}}
  "RESTORED_INITIAL=$initial"|Add-Content $out
 }
}else{"NO_EXE"|Add-Content $out}
Get-Content $out
