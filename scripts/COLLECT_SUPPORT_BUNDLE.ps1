param([Parameter(Mandatory=$true)][string]$SessionDir)
$ErrorActionPreference='Continue'
. (Join-Path $PSScriptRoot 'lib\TestHarness.Common.ps1')
$Root=Get-StockWatchRoot
if(-not(Test-Path $SessionDir)){throw "Session directory not found: $SessionDir"}
$support=Join-Path $SessionDir 'support';New-Item -ItemType Directory -Force -Path $support|Out-Null
# Only whitelist non-secret environment/config facts.
$machine=[ordered]@{
 collectedAt=(Get-Date).ToString('o');
 os=[System.Environment]::OSVersion.VersionString;
 os64=[Environment]::Is64BitOperatingSystem;
 process64=[Environment]::Is64BitProcess;
 powershell=$PSVersionTable.PSVersion.ToString();
 cpu=$env:PROCESSOR_IDENTIFIER;
 computer=$env:COMPUTERNAME;
 tools=[ordered]@{node=Get-CommandVersionLine 'node';npm=Get-CommandVersionLine 'npm';cargo=Get-CommandVersionLine 'cargo';rustc=Get-CommandVersionLine 'rustc';python=Get-CommandVersionLine 'python';git=Get-CommandVersionLine 'git'}
}
try{$os=Get-CimInstance Win32_OperatingSystem;$machine.ramGiB=[math]::Round($os.TotalVisibleMemorySize/1MB,2);$machine.windowsCaption=$os.Caption;$machine.windowsBuild=$os.BuildNumber}catch{}
Save-Json $machine (Join-Path $support 'machine.json')
try{Get-ComputerInfo -Property WindowsProductName,WindowsVersion,OsBuildNumber,OsArchitecture,CsSystemType,CsTotalPhysicalMemory|Format-List|Out-File (Join-Path $support 'computer-info.txt') -Encoding utf8}catch{}
try{netsh winhttp show proxy|Out-File (Join-Path $support 'winhttp-proxy.txt') -Encoding utf8}catch{}
try{Get-AuthenticodeSignature (Get-StockWatchReleaseExe) | Format-List * | Out-File (Join-Path $support 'exe-signature.txt') -Encoding utf8}catch{}
try{Get-WinEvent -FilterHashtable @{LogName='Application';StartTime=(Get-Date).AddHours(-2)} -ErrorAction Stop | Where-Object {$_.Message -match 'stockwatch|WebView2|tauri|Rust'} | Select-Object -First 100 TimeCreated,Id,LevelDisplayName,ProviderName,Message | Format-List | Out-File (Join-Path $support 'application-events.txt') -Encoding utf8}catch{}
try{Get-WinEvent -FilterHashtable @{LogName='System';StartTime=(Get-Date).AddHours(-2)} -ErrorAction Stop | Where-Object {$_.ProviderName -match 'Power-Troubleshooter|Kernel-Power'} | Select-Object -First 50 TimeCreated,Id,LevelDisplayName,ProviderName,Message | Format-List | Out-File (Join-Path $support 'power-events.txt') -Encoding utf8}catch{}
# Copy all test session evidence, excluding any nested support bundles.
Get-ChildItem $SessionDir -Force | Where-Object {$_.Name -ne 'support' -and $_.Name -notlike 'SUPPORT_BUNDLE_*.zip'} | ForEach-Object {
  try{Copy-Item $_.FullName -Destination $support -Recurse -Force}catch{}
}
# Hash key source/config/scripts so returned evidence identifies exactly what was tested.
$hashTargets=@('package.json','src-tauri\Cargo.toml','src-tauri\tauri.conf.json','src-tauri\src\lib.rs','scripts\QUALIFY_WINDOWS.ps1','scripts\RUN_ALL_AUTOMATED_TESTS.ps1')
$hashes=@();foreach($rel in $hashTargets){$p=Join-Path $Root $rel;if(Test-Path $p){$h=Get-FileHash $p -Algorithm SHA256;$hashes += "$($h.Hash.ToLower())  $rel"}}
$hashes|Set-Content (Join-Path $support 'tested-files-sha256.txt') -Encoding UTF8
$zip=Join-Path $SessionDir ("SUPPORT_BUNDLE_{0}.zip" -f (Get-Date -Format 'yyyyMMdd-HHmmss'))
Compress-Archive -Path (Join-Path $support '*') -DestinationPath $zip -CompressionLevel Optimal -Force
$h=Get-FileHash $zip -Algorithm SHA256
Write-Utf8File -Path (Join-Path $SessionDir 'SUPPORT_BUNDLE_SHA256.txt') -Text ("$($h.Hash.ToLower())  $([IO.Path]::GetFileName($zip))`n")
Write-Output $zip
