param([string]$OutputDir = '')
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'lib\TestHarness.Common.ps1')
[void](Initialize-StockWatchVisualStudioEnvironment)
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'PREFLIGHT'}
New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$result=[ordered]@{startedAt=(Get-Date).ToString('o');checks=[ordered]@{};info=[ordered]@{};overall='PASS'}
function Required([string]$name,[bool]$ok,[object]$detail){$script:result.checks[$name]=[ordered]@{status=if($ok){'PASS'}else{'FAIL'};detail=$detail};if(-not $ok){$script:result.overall='FAIL'}}
function Info([string]$name,[object]$detail){$script:result.info[$name]=$detail}
Required 'windows' (Test-IsWindowsHost) ([System.Environment]::OSVersion.VersionString)
Required 'powershell7' ($PSVersionTable.PSVersion.Major -ge 7) $PSVersionTable.PSVersion.ToString()
foreach($tool in @('node','npm','cargo','rustc')){$line=Get-CommandVersionLine $tool;Required "tool:$tool" ([bool]$line) $line}
if(Get-Command node -ErrorAction SilentlyContinue){
  try{$nv=[version]((& node --version).Trim().TrimStart('v'));$nodeOk=(($nv.Major -eq 20 -and $nv -ge [version]'20.19.0') -or ($nv.Major -ge 22 -and $nv -ge [version]'22.12.0'));Required 'node-supported-version' $nodeOk $nv.ToString()}catch{Required 'node-supported-version' $false $_.Exception.Message}
}
if(Get-Command rustc -ErrorAction SilentlyContinue){
  try{$rv=[version](((& rustc --version) -split ' ')[1] -replace '-.*$','');Required 'rust-supported-version' ($rv -ge [version]'1.90.0') $rv.ToString()}catch{Required 'rust-supported-version' $false $_.Exception.Message}
}
$py=(Get-Command python -ErrorAction SilentlyContinue);if(-not $py){$py=Get-Command python3 -ErrorAction SilentlyContinue};if(-not $py){$py=Get-Command py -ErrorAction SilentlyContinue}
Required 'tool:python' ([bool]$py) $(if($py){$py.Source}else{'Python 3 not found'})
Info 'git' (Get-CommandVersionLine 'git')
Info 'cl.exe' $(if(Get-Command cl.exe -ErrorAction SilentlyContinue){(Get-Command cl.exe).Source}else{'not currently in PATH'})
$vsInfo=Get-StockWatchVisualStudioInstallation
Required 'visual-studio-cpp' ([bool]$vsInfo) $(if($vsInfo){$vsInfo}else{'Visual Studio/Build Tools C++ workload not detected'})
Info 'webview2_registry_hint' ((@(
 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F1E7E5A5-4A3B-4B54-9B08-1B6D6A7C928B}',
 'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F1E7E5A5-4A3B-4B54-9B08-1B6D6A7C928B}',
 'HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F1E7E5A5-4A3B-4B54-9B08-1B6D6A7C928B}'
)|Where-Object{Test-Path $_}|Select-Object -First 1))
try{$root=(Get-StockWatchRoot);$driveName=[IO.Path]::GetPathRoot($root).Substring(0,1);$drive=Get-PSDrive -Name $driveName -ErrorAction Stop;Required 'free-disk-5gb' ($drive.Free -ge 5GB) ("{0} GiB" -f [math]::Round($drive.Free/1GB,2))}catch{Info 'free-disk-check' $_.Exception.Message}
$result.finishedAt=(Get-Date).ToString('o')
Save-Json $result (Join-Path $OutputDir 'preflight.json')
@($result.checks.GetEnumerator()|ForEach-Object{"$($_.Value.status)  $($_.Key)  $($_.Value.detail)"}) + @($result.info.GetEnumerator()|ForEach-Object{"INFO  $($_.Key)  $($_.Value)"}) | Set-Content (Join-Path $OutputDir 'preflight.txt') -Encoding UTF8
Get-Content (Join-Path $OutputDir 'preflight.txt')
Write-Host "PREFLIGHT_RESULT=$($result.overall)"
if($result.overall -ne 'PASS'){throw 'Preflight failed. See preflight.json for exact missing prerequisites.'}
