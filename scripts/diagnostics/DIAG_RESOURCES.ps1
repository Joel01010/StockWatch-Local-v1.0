param([string]$OutputDir='',[int]$Samples=15)
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-RESOURCES'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'resources.csv';'timestamp,pid,workingSetMiB,privateMiB,handles,threads,cpuSeconds,mainWindowHandle'|Set-Content $out -Encoding UTF8
$exe=Get-StockWatchReleaseExe;if(-not $exe){Write-Host 'NO_EXE';return}
try{$p=Start-StockWatchIsolatedProcess -Exe $exe -BaseDir $OutputDir;for($i=0;$i -lt $Samples;$i++){Start-Sleep 2;$p.Refresh();if($p.HasExited){break};"$((Get-Date).ToString('o')),$($p.Id),$([math]::Round($p.WorkingSet64/1MB,2)),$([math]::Round($p.PrivateMemorySize64/1MB,2)),$($p.HandleCount),$($p.Threads.Count),$([math]::Round($p.TotalProcessorTime.TotalSeconds,3)),$($p.MainWindowHandle)"|Add-Content $out};if(-not $p.HasExited){Stop-Process $p.Id -Force}}catch{"ERROR,$($_.Exception.Message)"|Add-Content $out}
Get-Content $out
