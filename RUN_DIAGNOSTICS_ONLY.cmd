@echo off
setlocal EnableExtensions
cd /d "%~dp0"
where pwsh.exe >nul 2>nul
if errorlevel 1 (
  echo ERROR: PowerShell 7 ^(pwsh.exe^) is required.
  exit /b 20
)
pwsh.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\DIAGNOSE_ALL.ps1"
exit /b %ERRORLEVEL%
