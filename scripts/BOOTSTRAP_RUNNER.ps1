param([switch]$IncludeInstalledAppTest)
$ErrorActionPreference='Continue'
$Root=Split-Path $PSScriptRoot -Parent
Set-Location $Root
$started=Get-Date
$master=Join-Path $PSScriptRoot 'RUN_ALL_AUTOMATED_TESTS.ps1'
$args=@('-NoLogo','-NoProfile','-ExecutionPolicy','Bypass','-File',$master)
if($IncludeInstalledAppTest){$args += '-IncludeInstalledAppTest'}
$proc=Start-Process -FilePath 'pwsh.exe' -ArgumentList $args -PassThru -Wait -NoNewWindow
$recent=Get-ChildItem (Join-Path $Root 'qualification-results') -Filter 'RETURN_THIS_FILE.txt' -File -Recurse -ErrorAction SilentlyContinue |
  Where-Object {$_.LastWriteTime -ge $started.AddMinutes(-1)} |
  Sort-Object LastWriteTime -Descending | Select-Object -First 1
if($recent){
  Get-Content $recent.FullName
  exit $proc.ExitCode
}
Write-Warning 'Master runner did not produce RETURN_THIS_FILE.txt. Running emergency diagnostics automatically.'
$fallback=Join-Path $PSScriptRoot 'DIAGNOSE_ALL.ps1'
$fb=Start-Process -FilePath 'pwsh.exe' -ArgumentList @('-NoLogo','-NoProfile','-ExecutionPolicy','Bypass','-File',$fallback) -PassThru -Wait -NoNewWindow
$recent=Get-ChildItem (Join-Path $Root 'qualification-results') -Filter 'SUPPORT_BUNDLE_*.zip' -File -Recurse -ErrorAction SilentlyContinue |
  Where-Object {$_.LastWriteTime -ge $started.AddMinutes(-1)} |
  Sort-Object LastWriteTime -Descending | Select-Object -First 1
if($recent){
  Write-Host "FINAL_RESULT=FAIL"
  Write-Host "RETURN_THIS_FILE=$($recent.FullName)"
}else{
  $err=Join-Path $Root 'qualification-results\BOOTSTRAP_FATAL_ERROR.txt'
  New-Item -ItemType Directory -Force -Path (Split-Path $err -Parent)|Out-Null
  "The master runner and emergency diagnostic collector both failed. Return this text file.`r`nMasterExit=$($proc.ExitCode)`r`nFallbackExit=$($fb.ExitCode)"|Set-Content $err -Encoding UTF8
  Write-Host "FINAL_RESULT=FAIL"
  Write-Host "RETURN_THIS_FILE=$err"
}
exit 1
