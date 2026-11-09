$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)
Write-Host "Phase 8: running the complete Windows qualification gate."
.\scripts\QUALIFY_WINDOWS.ps1
