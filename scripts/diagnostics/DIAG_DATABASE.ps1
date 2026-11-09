param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-DB'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'database.txt';"=== DATABASE / BACKUPS ==="|Set-Content $out -Encoding UTF8
$candidates=@(
 (Join-Path $env:APPDATA 'com.stockwatch.local\stockwatch.db'),
 (Join-Path $env:LOCALAPPDATA 'com.stockwatch.local\stockwatch.db'),
 (Join-Path $env:APPDATA 'StockWatch Local\stockwatch.db'),
 (Join-Path $env:LOCALAPPDATA 'StockWatch Local\stockwatch.db')
)|Where-Object{$_ -and (Test-Path $_)}|Select-Object -Unique
if(-not $candidates){"NO_USER_DATABASE_FOUND_IN_EXPECTED_PATHS"|Add-Content $out}
$pythonCmd=$null;$pythonPrefix=@();if(Get-Command python -ErrorAction SilentlyContinue){$pythonCmd='python'}elseif(Get-Command python3 -ErrorAction SilentlyContinue){$pythonCmd='python3'}elseif(Get-Command py -ErrorAction SilentlyContinue){$pythonCmd='py';$pythonPrefix=@('-3')}
$dbTool=Join-Path (Get-StockWatchRoot) 'tools\db_quick_check.py'
foreach($dbPath in $candidates){
 $db=Get-Item $dbPath;"DB=$($db.FullName) size=$($db.Length) modified=$($db.LastWriteTime.ToString('o'))"|Add-Content $out
 if($pythonCmd){ & $pythonCmd @pythonPrefix $dbTool $db.FullName 2>&1 | Add-Content $out }
 $dir=$db.Directory.FullName;Get-ChildItem $dir -File -ErrorAction SilentlyContinue|Where-Object{$_.Name -match 'backup|restore|corrupt|wal|shm'}|Select-Object Name,Length,LastWriteTime|Format-Table|Out-String|Add-Content $out
}
Get-Content $out
