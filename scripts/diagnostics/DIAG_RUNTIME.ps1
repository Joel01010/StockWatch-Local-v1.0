param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-RUNTIME'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'runtime.txt';"=== RUNTIME ==="|Set-Content $out -Encoding UTF8
$exe=Get-StockWatchReleaseExe
if(-not $exe){"NO_EXE: release/debug executable not found"|Add-Content $out;Get-Content $out;return}
"EXE=$exe"|Add-Content $out
try{$sig=Get-AuthenticodeSignature $exe;"SIGNATURE=$($sig.Status)"|Add-Content $out}catch{}
$before=Get-Date
try{$p=Start-StockWatchIsolatedProcess -Exe $exe -BaseDir $OutputDir;Start-Sleep 5;$p.Refresh();"PID=$($p.Id) HasExited=$($p.HasExited) MainWindow=$($p.MainWindowHandle) WorkingSetMiB=$([math]::Round($p.WorkingSet64/1MB,2)) PrivateMiB=$([math]::Round($p.PrivateMemorySize64/1MB,2)) Handles=$($p.HandleCount) Threads=$($p.Threads.Count)"|Add-Content $out;if(-not $p.HasExited){Stop-Process -Id $p.Id -Force}}catch{"START_FAIL=$($_.Exception.Message)"|Add-Content $out}
try{Get-WinEvent -FilterHashtable @{LogName='Application';StartTime=$before} -ErrorAction Stop|Where-Object{$_.Message -match 'stockwatch|WebView2|tauri|Rust'}|Select-Object TimeCreated,Id,LevelDisplayName,ProviderName,Message|Format-List|Out-String|Add-Content $out}catch{}
Get-Content $out
