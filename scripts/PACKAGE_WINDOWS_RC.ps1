param([switch]$SkipQualification)
$ErrorActionPreference='Stop'
$Root=Split-Path $PSScriptRoot -Parent
Set-Location $Root
if(-not $IsWindows){throw 'Windows RC packaging must run on Windows.'}
if(-not $SkipQualification){.\scripts\QUALIFY_WINDOWS.ps1}
$dist=Join-Path $Root 'dist';$stage=Join-Path $dist 'StockWatchLocal_v1.0_RC1_Windows'
Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue;New-Item -ItemType Directory -Force -Path $stage|Out-Null
$exe=Join-Path $Root 'src-tauri\target\release\stockwatch-local.exe';if(-not(Test-Path $exe)){throw 'Release executable missing.'}
Copy-Item $exe $stage
$installer=Get-ChildItem 'src-tauri\target\release\bundle\nsis\*.exe' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if($installer){Copy-Item $installer.FullName $stage}
Copy-Item README.md,CHANGELOG.md,KNOWN_ISSUES.md,PHASE_REPORT.md,TEST_RESULTS.md -Destination $stage
Copy-Item docs\WINDOWS_QUALIFICATION.md -Destination $stage
$hashes=Get-ChildItem $stage -File | Get-FileHash -Algorithm SHA256 | ForEach-Object {"$($_.Hash.ToLower())  $([IO.Path]::GetFileName($_.Path))"}
$hashes | Set-Content (Join-Path $stage 'SHA256SUMS.txt')
$zip=Join-Path $dist 'StockWatchLocal_v1.0_RC1_Windows.zip';Remove-Item $zip -Force -ErrorAction SilentlyContinue;Compress-Archive -Path "$stage\*" -DestinationPath $zip -CompressionLevel Optimal
Write-Host "Created $zip"
Get-FileHash $zip -Algorithm SHA256
