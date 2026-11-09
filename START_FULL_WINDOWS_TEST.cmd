@echo off
setlocal EnableExtensions
cd /d "%~dp0"
where pwsh.exe >nul 2>nul
if errorlevel 1 (
  if not exist "qualification-results" mkdir "qualification-results"
  >"qualification-results\BOOTSTRAP_MISSING_PWSH.txt" echo PowerShell 7 ^(pwsh.exe^) is required.
  echo FINAL_RESULT=FAIL
  echo RETURN_THIS_FILE=%~dp0qualification-results\BOOTSTRAP_MISSING_PWSH.txt
  exit /b 20
)
pwsh.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\BOOTSTRAP_RUNNER.ps1" -IncludeInstalledAppTest
exit /b %ERRORLEVEL%
