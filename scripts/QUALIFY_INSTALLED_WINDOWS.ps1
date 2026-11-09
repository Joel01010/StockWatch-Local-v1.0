param(
  [string]$InstallerPath = '',
  [switch]$AllowInstall,
  [switch]$KeepInstalled,
  [string]$ResultsDir = ''
)
$ErrorActionPreference='Stop'
$Root=Split-Path $PSScriptRoot -Parent
Set-Location $Root
if(-not $IsWindows){throw 'Installed-app qualification requires Windows.'}
if(-not $AllowInstall){throw 'Installed-app test intentionally changes Windows. Run only through START_FULL_WINDOWS_TEST.cmd or pass -AllowInstall.'}
$results=if($ResultsDir){$ResultsDir}else{Join-Path $Root 'qualification-results\installed'}
New-Item -ItemType Directory -Force -Path $results|Out-Null
$summary=[ordered]@{startedAt=(Get-Date).ToString('o');overall='FAIL';installer=$null;installedExe=$null;notificationApi='NOT_RUN';cleanup='NOT_RUN'}
function Find-Entry {
  $roots=@('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*')
  return Get-ItemProperty $roots -ErrorAction SilentlyContinue | Where-Object {$_.DisplayName -like 'StockWatch Local*'} | Select-Object -First 1
}
function Invoke-Uninstall([object]$Entry){
  $cmd=$Entry.QuietUninstallString;if(-not $cmd){$cmd=$Entry.UninstallString}
  if(-not $cmd){throw 'No uninstall command in registry entry.'}
  $exe=$null;$args=''
  if($cmd -match '^\s*"([^"]+)"\s*(.*)$'){$exe=$matches[1];$args=$matches[2]}elseif($cmd -match '^\s*(\S+)\s*(.*)$'){$exe=$matches[1];$args=$matches[2]}
  if(-not $exe -or -not(Test-Path $exe)){throw "Uninstaller not found from: $cmd"}
  if($args -notmatch '(^|\s)/S(\s|$)'){$args=("$args /S").Trim()}
  $p=Start-Process -FilePath $exe -ArgumentList $args -PassThru -Wait
  if($p.ExitCode -ne 0){throw "Uninstaller exited $($p.ExitCode)"}
}
$pre=Find-Entry
if($pre){$summary.preExistingInstall=$pre.DisplayName;$summary|ConvertTo-Json -Depth 6|Set-Content (Join-Path $results 'installed-summary.json') -Encoding UTF8;throw 'A StockWatch Local installation already exists. Refusing to overwrite it automatically.'}
$installedByThisRun=$false
try{
  if(-not $InstallerPath){$InstallerPath=(Get-ChildItem 'src-tauri\target\release\bundle\nsis\*.exe' -ErrorAction Stop | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName}
  if(-not(Test-Path $InstallerPath)){throw "Installer not found: $InstallerPath"}
  $summary.installer=$InstallerPath
  $p=Start-Process -FilePath $InstallerPath -ArgumentList '/S' -PassThru -Wait
  if($p.ExitCode -ne 0){throw "Installer exited $($p.ExitCode)"}
  $installedByThisRun=$true
  $entry=Find-Entry
  if(-not $entry){throw 'Installer completed but no StockWatch uninstall registry entry was found.'}
  $exe=$null
  if($entry.InstallLocation){$installLocation=($entry.InstallLocation -replace '^\s*["'']|["'']\s*$','').Trim();$candidate=Join-Path $installLocation 'stockwatch-local.exe';if(Test-Path $candidate){$exe=$candidate}}
  if(-not $exe){$exe=(Get-ChildItem $env:LOCALAPPDATA -Filter 'stockwatch-local.exe' -File -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1).FullName}
  if(-not $exe){throw 'Installed StockWatch executable could not be located.'}
  $summary.installedExe=$exe
  $report=Join-Path $results 'native-notification.txt'
  $old=@{mode=$env:STOCKWATCH_QUALIFICATION_MODE;notify=$env:STOCKWATCH_QUALIFICATION_NOTIFY;report=$env:STOCKWATCH_QUALIFICATION_REPORT;exit=$env:STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS}
  try{$env:STOCKWATCH_QUALIFICATION_MODE='1';$env:STOCKWATCH_QUALIFICATION_NOTIFY='1';$env:STOCKWATCH_QUALIFICATION_REPORT=$report;$env:STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS='1800';$p=Start-Process -FilePath $exe -PassThru}finally{$env:STOCKWATCH_QUALIFICATION_MODE=$old.mode;$env:STOCKWATCH_QUALIFICATION_NOTIFY=$old.notify;$env:STOCKWATCH_QUALIFICATION_REPORT=$old.report;$env:STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS=$old.exit}
  if(-not $p.WaitForExit(10000)){Stop-Process -Id $p.Id -Force;throw 'Installed notification smoke process did not exit.'}
  $text=if(Test-Path $report){Get-Content $report -Raw}else{''}
  if($text -notmatch 'notification=ok'){throw "Native notification API failed: $text"}
  $summary.notificationApi='PASS'
  $summary.notificationVisual='UNVERIFIED_AUTOMATICALLY'
  $summary.overall='PASS'
} finally {
  if($installedByThisRun -and -not $KeepInstalled){
    try{$entry=Find-Entry;if($entry){Invoke-Uninstall $entry};$summary.cleanup='PASS'}catch{$summary.cleanup="FAIL: $($_.Exception.Message)";$summary.overall='FAIL'}
  } elseif($KeepInstalled){$summary.cleanup='SKIPPED_KEEP_INSTALLED'}
  $summary.finishedAt=(Get-Date).ToString('o')
  $summary|ConvertTo-Json -Depth 8|Set-Content (Join-Path $results 'installed-summary.json') -Encoding UTF8
}
Write-Host "INSTALLED_TEST_RESULT=$($summary.overall)"
Write-Host "INSTALLED_TEST_OUTPUT=$results"
if($summary.overall -ne 'PASS'){throw 'Installed-app qualification failed. See installed-summary.json.'}
