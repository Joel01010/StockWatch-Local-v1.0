param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-TRAY'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'tray-webview.txt';"=== TRAY / WEBVIEW LIFECYCLE ==="|Set-Content $out -Encoding UTF8
$exe=Get-StockWatchReleaseExe;if(-not $exe){"NO_EXE"|Add-Content $out;Get-Content $out;return}
try{
 Add-Type @'
using System; using System.Runtime.InteropServices;
public static class SWDiagNative { [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd,uint Msg,IntPtr wParam,IntPtr lParam); }
'@ -ErrorAction SilentlyContinue
 foreach($mode in @('','dashboard','mini')){
  $p=Start-StockWatchIsolatedProcess -Exe $exe -BaseDir $OutputDir -ExtraEnvironment @{STOCKWATCH_QUALIFICATION_WINDOW=$mode}
  Start-Sleep 4;$p.Refresh();"MODE='$mode' pid=$($p.Id) exited=$($p.HasExited) hwnd=$($p.MainWindowHandle)"|Add-Content $out
  if(-not $p.HasExited -and $p.MainWindowHandle -ne 0){[SWDiagNative]::PostMessage($p.MainWindowHandle,0x0010,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null;Start-Sleep 3;$p.Refresh();"AFTER_CLOSE exited=$($p.HasExited) hwnd=$($p.MainWindowHandle)"|Add-Content $out}
  if(-not $p.HasExited){Stop-Process $p.Id -Force}
 }
}catch{"ERROR=$($_.Exception.Message)"|Add-Content $out}
Get-Content $out
