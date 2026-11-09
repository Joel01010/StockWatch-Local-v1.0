param(
  [switch]$SkipBundle,
  [switch]$SkipNativeSmoke,
  [switch]$KeepArtifacts,
  [string]$ResultsDir = ''
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\TestHarness.Common.ps1')
[void](Initialize-StockWatchVisualStudioEnvironment)
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root
if (-not $IsWindows) { throw 'Phase 8 native qualification must run on Windows.' }

$Stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$Results = if($ResultsDir){$ResultsDir}else{Join-Path $Root "qualification-results\$Stamp"}
New-Item -ItemType Directory -Force -Path $Results | Out-Null
$Transcript = Join-Path $Results 'qualification-transcript.txt'
Start-Transcript -Path $Transcript -Force | Out-Null
$Summary = [ordered]@{ startedAt=(Get-Date).ToString('o'); computer=$env:COMPUTERNAME; windows=$true; checks=@{}; resources=@{} }

function Pass([string]$Name,[object]$Detail=$true) { $Summary.checks[$Name]=[ordered]@{status='PASS';detail=$Detail}; Write-Host "PASS: $Name" -ForegroundColor Green }
function Warn([string]$Name,[object]$Detail) { $Summary.checks[$Name]=[ordered]@{status='WARN';detail=$Detail}; Write-Warning "$Name - $Detail" }
function Fail([string]$Name,[object]$Detail) { $Summary.checks[$Name]=[ordered]@{status='FAIL';detail=$Detail}; throw "$Name - $Detail" }
function Invoke-Logged([string]$Name,[scriptblock]$Block) {
  Write-Host "`n=== $Name ===" -ForegroundColor Cyan
  $global:LASTEXITCODE = 0
  & $Block
  if ($LASTEXITCODE -ne $null -and $LASTEXITCODE -ne 0) { Fail $Name "exit code $LASTEXITCODE" }
  Pass $Name
}
function Require-Command([string]$Name) { if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) { Fail "tool:$Name" 'not found in PATH' } else { Pass "tool:$Name" ((Get-Command $Name).Source) } }
function Wait-Window([System.Diagnostics.Process]$P,[int]$Seconds=15) {
  $deadline=(Get-Date).AddSeconds($Seconds)
  do { Start-Sleep -Milliseconds 250; $P.Refresh(); if ($P.HasExited) { return [IntPtr]::Zero }; $handle = Get-StockWatchMainWindowHandle $P; if ($handle -ne [IntPtr]::Zero) { return $handle } } while ((Get-Date) -lt $deadline)
  return [IntPtr]::Zero
}
function Wait-WindowGone([System.Diagnostics.Process]$P,[int]$Seconds=10) {
  $deadline=(Get-Date).AddSeconds($Seconds)
  do { Start-Sleep -Milliseconds 250; $P.Refresh(); if ($P.HasExited) { return $false }; if ((Get-StockWatchMainWindowHandle $P) -eq [IntPtr]::Zero) { return $true } } while ((Get-Date) -lt $deadline)
  return $false
}
function Stop-TestProcess([System.Diagnostics.Process]$P) { if ($P -and -not $P.HasExited) { Stop-Process -Id $P.Id -Force -ErrorAction SilentlyContinue; $P.WaitForExit(5000) | Out-Null } }
function With-ChildEnvironment([hashtable]$Vars,[scriptblock]$Block) {
  $old=@{}
  foreach($k in $Vars.Keys) { $old[$k]=[Environment]::GetEnvironmentVariable($k,'Process'); [Environment]::SetEnvironmentVariable($k,[string]$Vars[$k],'Process') }
  try { & $Block } finally { foreach($k in $Vars.Keys) { [Environment]::SetEnvironmentVariable($k,$old[$k],'Process') } }
}
function Start-Isolated([string]$Exe,[string]$Mode='',[hashtable]$Extra=@{}) {
  $id=[Guid]::NewGuid().ToString('N')
  $base=Join-Path $Results "runtime-$id"
  $appdata=Join-Path $base 'AppData\Roaming'; $local=Join-Path $base 'AppData\Local'
  New-Item -ItemType Directory -Force -Path $appdata,$local | Out-Null
  $vars=@{ APPDATA=$appdata; LOCALAPPDATA=$local; STOCKWATCH_QUALIFICATION_MODE='1'; STOCKWATCH_QUALIFICATION_WINDOW=$Mode }
  foreach($k in $Extra.Keys) { $vars[$k]=$Extra[$k] }
  return (With-ChildEnvironment $vars { Start-Process -FilePath $Exe -PassThru })
}

try {
  Require-Command node; Require-Command npm; Require-Command cargo; Require-Command rustc
  $rustLine = (& rustc --version).Trim(); $nodeLine = (& node --version).Trim()
  $rustVersion=[version](($rustLine -split ' ')[1] -replace '-.*$','')
  if ($rustVersion -lt [version]'1.90.0') { Fail 'rust-msrv' "Rust >=1.90 required by current Tauri plugins; found $rustLine" } else { Pass 'rust-msrv' $rustLine }
  Pass 'node-version' $nodeLine
  $os=Get-CimInstance Win32_OperatingSystem; $Summary.os=[ordered]@{caption=$os.Caption;version=$os.Version;build=$os.BuildNumber;arch=$env:PROCESSOR_ARCHITECTURE}
  if (-not [Environment]::Is64BitOperatingSystem) { Fail 'windows-64bit' '64-bit Windows required for this qualification profile' } else { Pass 'windows-64bit' $Summary.os }

  if (-not (Test-Path node_modules)) { Invoke-Logged 'npm-install' { npm install --no-audit --no-fund } }
  Invoke-Logged 'software-regression' { .\scripts\CHECK_SOFTWARE_ONLY.ps1 }
  Invoke-Logged 'phase8-source-check' { npm run check:phase8 }
  Invoke-Logged 'release-reference-tests' { npm run test:release }
  Invoke-Logged 'cargo-test' { cargo test --manifest-path src-tauri/Cargo.toml }
  Invoke-Logged 'cargo-fmt' { cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check }
  Invoke-Logged 'cargo-clippy' { cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings }
  Invoke-Logged 'frontend-production-build' { npm run build }

  if (-not $SkipBundle) { Invoke-Logged 'tauri-nsis-build' { npm run tauri build -- --bundles nsis } }
  $Exe = Join-Path $Root 'src-tauri\target\release\stockwatch-local.exe'
  if (-not (Test-Path $Exe)) { Fail 'release-executable' "not found: $Exe" } else { Pass 'release-executable' $Exe }

  if (-not $SkipNativeSmoke) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class StockWatchNative {
  [DllImport("user32.dll", SetLastError=true)] public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
}
'@
    # 1. Tray-only startup: no WebView/window should exist.
    $p=Start-Isolated $Exe
    try {
      Start-Sleep -Seconds 4; $p.Refresh()
      if ($p.HasExited) { Fail 'tray-only-startup' "process exited with $($p.ExitCode)" }
      if ((Get-StockWatchMainWindowHandle $p) -ne [IntPtr]::Zero) { Fail 'tray-only-startup' 'unexpected visible main window at startup' }
      Pass 'tray-only-startup' "pid=$($p.Id), no WebView window"
      $Summary.resources.trayOnly=[ordered]@{workingSetMiB=[math]::Round($p.WorkingSet64/1MB,2); privateMiB=[math]::Round($p.PrivateMemorySize64/1MB,2); handles=$p.HandleCount; threads=$p.Threads.Count}
    } finally { Stop-TestProcess $p }

    # 2. Open dashboard automatically, close it with WM_CLOSE, backend must remain alive.
    $p=Start-Isolated $Exe 'dashboard'
    try {
      $h=Wait-Window $p 20
      if ($h -eq [IntPtr]::Zero) { Fail 'dashboard-open' 'dashboard window did not appear' }
      $p.Refresh(); $Summary.resources.dashboardOpen=[ordered]@{workingSetMiB=[math]::Round($p.WorkingSet64/1MB,2); privateMiB=[math]::Round($p.PrivateMemorySize64/1MB,2); handles=$p.HandleCount; threads=$p.Threads.Count}
      Pass 'dashboard-open' "handle=$h"
      [StockWatchNative]::PostMessage($h,0x0010,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
      if (-not (Wait-WindowGone $p 15)) { Fail 'dashboard-destroy' 'window did not close or process exited' }
      Start-Sleep -Seconds 3; $p.Refresh()
      if ($p.HasExited) { Fail 'tray-survives-dashboard-close' 'backend exited after last WebView closed' }
      $Summary.resources.trayAfterDashboardClose=[ordered]@{workingSetMiB=[math]::Round($p.WorkingSet64/1MB,2); privateMiB=[math]::Round($p.PrivateMemorySize64/1MB,2); handles=$p.HandleCount; threads=$p.Threads.Count}
      Pass 'tray-survives-dashboard-close' $Summary.resources.trayAfterDashboardClose
    } finally { Stop-TestProcess $p }

    # 3. Quick View can be created and destroyed without killing backend.
    $p=Start-Isolated $Exe 'mini'
    try {
      $h=Wait-Window $p 20
      if ($h -eq [IntPtr]::Zero) { Fail 'mini-open' 'Quick View window did not appear' }
      Pass 'mini-open' "handle=$h"
      [StockWatchNative]::PostMessage($h,0x0010,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
      if (-not (Wait-WindowGone $p 15)) { Fail 'mini-destroy' 'Quick View did not close or backend exited' }
      Pass 'mini-destroy' 'backend remained alive'
    } finally { Stop-TestProcess $p }

    # 4. Explicit coded exit must not be swallowed by the last-window lifetime guard.
    $p=Start-Isolated $Exe '' @{STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS='1200'}
    if (-not $p.WaitForExit(10000)) { Stop-TestProcess $p; Fail 'explicit-exit' 'app.exit(0) did not terminate process' }
    if ($p.ExitCode -ne 0) { Fail 'explicit-exit' "exit code $($p.ExitCode)" } else { Pass 'explicit-exit' 0 }

    # 5. Native autostart plugin can register/unregister and MUST restore the pre-test state.
    $reportStatus=Join-Path $Results 'autostart-status-before.txt'
    $p=Start-Isolated $Exe '' @{STOCKWATCH_QUALIFICATION_AUTOSTART='status';STOCKWATCH_QUALIFICATION_REPORT=$reportStatus;STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS='1200'}
    if (-not $p.WaitForExit(10000)) { Stop-TestProcess $p; Fail 'autostart-status-before' 'qualification process did not exit' }
    $statusText=if(Test-Path $reportStatus){Get-Content $reportStatus -Raw}else{''}
    $initialAutostart = $statusText -match 'autostart_enabled=true'
    Pass 'autostart-status-before' $statusText.Trim()
    try {
      $reportEnable=Join-Path $Results 'autostart-enable.txt'
      $p=Start-Isolated $Exe '' @{STOCKWATCH_QUALIFICATION_AUTOSTART='enable';STOCKWATCH_QUALIFICATION_REPORT=$reportEnable;STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS='1200'}
      if (-not $p.WaitForExit(10000)) { Stop-TestProcess $p; Fail 'autostart-enable' 'qualification process did not exit' }
      $text=if(Test-Path $reportEnable){Get-Content $reportEnable -Raw}else{''}
      if ($text -notmatch 'autostart_action=enable:ok' -or $text -notmatch 'autostart_enabled=true') { Fail 'autostart-enable' $text } else { Pass 'autostart-enable' $text.Trim() }
      $reportDisable=Join-Path $Results 'autostart-disable.txt'
      $p=Start-Isolated $Exe '' @{STOCKWATCH_QUALIFICATION_AUTOSTART='disable';STOCKWATCH_QUALIFICATION_REPORT=$reportDisable;STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS='1200'}
      if (-not $p.WaitForExit(10000)) { Stop-TestProcess $p; Fail 'autostart-disable' 'qualification process did not exit' }
      $text=if(Test-Path $reportDisable){Get-Content $reportDisable -Raw}else{''}
      if ($text -notmatch 'autostart_action=disable:ok' -or $text -notmatch 'autostart_enabled=false') { Fail 'autostart-disable' $text } else { Pass 'autostart-disable' $text.Trim() }
    } finally {
      $restoreAction = if($initialAutostart){'enable'}else{'disable'}
      $restoreReport=Join-Path $Results 'autostart-restore.txt'
      $p=Start-Isolated $Exe '' @{STOCKWATCH_QUALIFICATION_AUTOSTART=$restoreAction;STOCKWATCH_QUALIFICATION_REPORT=$restoreReport;STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS='1200'}
      if (-not $p.WaitForExit(10000)) { Stop-TestProcess $p }
      $restoreText=if(Test-Path $restoreReport){Get-Content $restoreReport -Raw}else{''}
      $wanted = if($initialAutostart){'autostart_enabled=true'}else{'autostart_enabled=false'}
      if($restoreText -match [regex]::Escape($wanted)){Pass 'autostart-restore' $restoreText.Trim()}else{Warn 'autostart-restore' $restoreText.Trim()}
    }
  }

  $Summary.finishedAt=(Get-Date).ToString('o'); $Summary.overall='PASS'
} catch {
  $Summary.finishedAt=(Get-Date).ToString('o'); $Summary.overall='FAIL'; $Summary.error=$_.Exception.Message
  throw
} finally {
  $Summary | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $Results 'qualification-summary.json') -Encoding UTF8
  Stop-Transcript | Out-Null
  Write-Host "Qualification results: $Results"
}

