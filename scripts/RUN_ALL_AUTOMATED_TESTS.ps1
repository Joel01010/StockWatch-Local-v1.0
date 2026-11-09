param([switch]$IncludeInstalledAppTest)
$ErrorActionPreference='Continue'
. (Join-Path $PSScriptRoot 'lib\TestHarness.Common.ps1')
$Root=Get-StockWatchRoot
Set-Location $Root
$Session=New-TestSession -Prefix 'AUTO'
$env:STOCKWATCH_TEST_SESSION=$Session
$Transcript=Join-Path $Session 'MASTER_TRANSCRIPT.txt'
try{Start-Transcript -Path $Transcript -Force|Out-Null}catch{}
$summary=[ordered]@{startedAt=(Get-Date).ToString('o');session=$Session;mode=if($IncludeInstalledAppTest){'full-including-install'}else{'safe-no-install'};steps=@();diagnostics=@();overall='PASS';supportBundle=$null}
Write-Host "STOCKWATCH TEST HARNESS" -ForegroundColor Cyan
Write-Host "SESSION=$Session"
Write-Host "RULE: do not choose diagnostics manually; this script runs them automatically on failure."

function AddStep([string]$Name,[scriptblock]$Block){
  $r=Invoke-CapturedCommand -Name $Name -Script $Block -OutputDir $Session -NeverThrow
  $script:summary.steps += $r
  if($r.status -ne 'PASS'){$script:summary.overall='FAIL'}
  return $r
}

$pre=AddStep '01_PREFLIGHT' { & (Join-Path $PSScriptRoot 'PREFLIGHT_WINDOWS.ps1') -OutputDir $Session }
if($pre.status -eq 'PASS'){
  AddStep '02_WINDOWS_QUALIFICATION' { & (Join-Path $PSScriptRoot 'QUALIFY_WINDOWS.ps1') -ResultsDir (Join-Path $Session 'native-qualification') } | Out-Null
  if($IncludeInstalledAppTest -and (Get-LatestNsisInstaller)){
    AddStep '03_INSTALLED_APP_QUALIFICATION' { & (Join-Path $PSScriptRoot 'QUALIFY_INSTALLED_WINDOWS.ps1') -AllowInstall -ResultsDir (Join-Path $Session 'installed-qualification') } | Out-Null
  } elseif($IncludeInstalledAppTest) {
    $summary.overall='FAIL'
    $summary.steps += [ordered]@{name='03_INSTALLED_APP_QUALIFICATION';status='FAIL';exitCode=2;error='NSIS installer not found';log=$null}
  }
}else{
  Write-Warning 'Preflight failed. Build/runtime qualification skipped; diagnostics will identify prerequisites.'
}

# Diagnostics always run. This is intentional: a weak operator/model never has to choose the right one.
$diagDir=Join-Path $Session 'diagnostics';New-Item -ItemType Directory -Force -Path $diagDir|Out-Null
$diagScripts=@(
 'DIAG_TOOLCHAIN.ps1','DIAG_NETWORK.ps1','DIAG_MARKET_DATA.ps1','DIAG_WEBVIEW2.ps1',
 'DIAG_BUILD.ps1','DIAG_RUNTIME.ps1','DIAG_TRAY_WEBVIEW.ps1','DIAG_AUTOSTART.ps1',
 'DIAG_NOTIFICATIONS.ps1','DIAG_DATABASE.ps1','DIAG_RESOURCES.ps1','DIAG_SECURITY.ps1','DIAG_INSTALLER.ps1'
)
foreach($name in $diagScripts){
  $path=Join-Path $PSScriptRoot "diagnostics\$name"
  $r=Invoke-CapturedCommand -Name ("DIAG_"+[IO.Path]::GetFileNameWithoutExtension($name)) -Script { & $path -OutputDir $diagDir } -OutputDir $diagDir -NeverThrow
  $summary.diagnostics += $r
}

Save-Json $summary (Join-Path $Session 'MASTER_SUMMARY.json')
try{
  $bundle=& (Join-Path $PSScriptRoot 'COLLECT_SUPPORT_BUNDLE.ps1') -SessionDir $Session
  $summary.supportBundle=($bundle|Select-Object -Last 1).ToString().Trim()
}catch{
  $summary.overall='FAIL';$summary.supportBundle=$null
  "Support bundle failed: $($_.Exception.Message)"|Add-Content (Join-Path $Session 'SUPPORT_BUNDLE_ERROR.txt')
}
$summary.finishedAt=(Get-Date).ToString('o')
Save-Json $summary (Join-Path $Session 'MASTER_SUMMARY.json')
if($summary.supportBundle){Write-ReturnFile -SessionDir $Session -BundlePath $summary.supportBundle -Overall $summary.overall}
try{Stop-Transcript|Out-Null}catch{}
Write-Host "FINAL_RESULT=$($summary.overall)" -ForegroundColor $(if($summary.overall -eq 'PASS'){'Green'}else{'Red'})
Write-Host "SESSION=$Session"
if($summary.supportBundle){Write-Host "RETURN_THIS_FILE=$($summary.supportBundle)" -ForegroundColor Yellow}
if($summary.overall -eq 'PASS'){exit 0}else{exit 1}
