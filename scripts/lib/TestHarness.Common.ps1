Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

function Get-StockWatchRoot {
    return (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent)
}

function Test-IsWindowsHost {
    return [System.Environment]::OSVersion.Platform -eq [System.PlatformID]::Win32NT
}

function New-TestSession {
    param([string]$Prefix = 'AUTO')
    $root = Get-StockWatchRoot
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $dir = Join-Path $root "qualification-results\$Prefix-$stamp"
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    return $dir
}

function Write-Utf8File {
    param([string]$Path, [string]$Text)
    $parent = Split-Path $Path -Parent
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false))
}

function Save-Json {
    param([object]$Value, [string]$Path, [int]$Depth = 12)
    Write-Utf8File -Path $Path -Text (($Value | ConvertTo-Json -Depth $Depth) + [Environment]::NewLine)
}

function Invoke-CapturedCommand {
    param(
        [Parameter(Mandatory=$true)][string]$Name,
        [Parameter(Mandatory=$true)][scriptblock]$Script,
        [Parameter(Mandatory=$true)][string]$OutputDir,
        [switch]$NeverThrow
    )
    $safe = ($Name -replace '[^A-Za-z0-9_.-]','_')
    $log = Join-Path $OutputDir "$safe.log"
    $started = Get-Date
    $status = 'PASS'
    $exitCode = 0
    $errorText = $null
    try {
        $global:LASTEXITCODE = 0
        & $Script *>&1 | Tee-Object -FilePath $log | Out-Host
        $nativeExit = $null
        $v = Get-Variable LASTEXITCODE -ErrorAction SilentlyContinue
        if ($v) { $nativeExit = $v.Value }
        if ($nativeExit -is [int] -and $nativeExit -ne 0) {
            throw "Command exited with code $nativeExit"
        }
    } catch {
        $status = 'FAIL'
        $v = Get-Variable LASTEXITCODE -ErrorAction SilentlyContinue
        $nativeExit = if ($v) { $v.Value } else { $null }
        $exitCode = if ($nativeExit -is [int] -and $nativeExit -ne 0) { $nativeExit } else { 1 }
        $errorText = $_.Exception.Message
        "ERROR: $errorText" | Add-Content -Path $log -Encoding UTF8
        if (-not $NeverThrow) { throw }
    }
    $ended = Get-Date
    return [ordered]@{
        name = $Name
        status = $status
        exitCode = $exitCode
        startedAt = $started.ToString('o')
        endedAt = $ended.ToString('o')
        durationSeconds = [math]::Round(($ended - $started).TotalSeconds, 3)
        log = $log
        error = $errorText
    }
}

function Get-CommandVersionLine {
    param([string]$Command, [string[]]$Arguments = @('--version'))
    $cmd = Get-Command $Command -ErrorAction SilentlyContinue
    if (-not $cmd) { return $null }
    try { return ((& $Command @Arguments 2>&1 | Select-Object -First 1).ToString()).Trim() } catch { return "present; version probe failed: $($_.Exception.Message)" }
}

function Get-StockWatchReleaseExe {
    $root = Get-StockWatchRoot
    $candidates = @(
        (Join-Path $root 'src-tauri\target\release\stockwatch-local.exe'),
        (Join-Path $root 'src-tauri\target\debug\stockwatch-local.exe')
    )
    foreach ($candidate in $candidates) { if (Test-Path $candidate) { return $candidate } }
    return $null
}

function Get-LatestNsisInstaller {
    $root = Get-StockWatchRoot
    $dir = Join-Path $root 'src-tauri\target\release\bundle\nsis'
    if (-not (Test-Path $dir)) { return $null }
    $item = Get-ChildItem $dir -Filter '*.exe' -File -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($item) { return $item.FullName }
    return $null
}

function Write-ReturnFile {
    param([string]$SessionDir, [string]$BundlePath, [string]$Overall)
    $text = @"
STOCKWATCH LOCAL TEST RESULT
FINAL_RESULT=$Overall
RETURN_THIS_FILE=$BundlePath

Do not edit the ZIP.
Return the ZIP file named above for diagnosis.
"@
    Write-Utf8File -Path (Join-Path $SessionDir 'RETURN_THIS_FILE.txt') -Text $text
}

function Start-StockWatchIsolatedProcess {
    param(
        [Parameter(Mandatory=$true)][string]$Exe,
        [Parameter(Mandatory=$true)][string]$BaseDir,
        [hashtable]$ExtraEnvironment = @{}
    )
    $id=[Guid]::NewGuid().ToString('N')
    $profile=Join-Path $BaseDir "isolated-$id"
    $appdata=Join-Path $profile 'AppData\Roaming'
    $localappdata=Join-Path $profile 'AppData\Local'
    New-Item -ItemType Directory -Force -Path $appdata,$localappdata|Out-Null
    $vars=[ordered]@{APPDATA=$appdata;LOCALAPPDATA=$localappdata;STOCKWATCH_QUALIFICATION_MODE='1'}
    foreach($k in $ExtraEnvironment.Keys){$vars[$k]=[string]$ExtraEnvironment[$k]}
    $old=@{}
    foreach($k in $vars.Keys){$old[$k]=[Environment]::GetEnvironmentVariable($k,'Process');[Environment]::SetEnvironmentVariable($k,$vars[$k],'Process')}
    try{return Start-Process -FilePath $Exe -PassThru}
    finally{foreach($k in $vars.Keys){[Environment]::SetEnvironmentVariable($k,$old[$k],'Process')}}
}

function Test-StockWatchVisualStudioInstallation {
    param([string]$InstallationPath)
    if (-not $InstallationPath -or -not (Test-Path -LiteralPath $InstallationPath -PathType Container)) {
        return $false
    }
    $vsDevCmd = Join-Path $InstallationPath 'VC\Auxiliary\Build\VsDevCmd.bat'
    $vcVars = Join-Path $InstallationPath 'VC\Auxiliary\Build\vcvarsall.bat'
    if (-not (Test-Path -LiteralPath $vsDevCmd) -and -not (Test-Path -LiteralPath $vcVars)) {
        return $false
    }
    $msvcRoot = Join-Path $InstallationPath 'VC\Tools\MSVC'
    if (-not (Test-Path -LiteralPath $msvcRoot -PathType Container)) {
        return $false
    }
    $cl = Get-ChildItem -Path $msvcRoot -Recurse -Filter 'cl.exe' -File -ErrorAction SilentlyContinue | Select-Object -First 1
    $link = Get-ChildItem -Path $msvcRoot -Recurse -Filter 'link.exe' -File -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $cl -or -not $link) {
        return $false
    }
    $sdkFound = $false
    $kitRoots = @()
    if ($env:ProgramFiles) { $kitRoots += (Join-Path $env:ProgramFiles 'Windows Kits') }
    if (${env:ProgramFiles(x86)}) { $kitRoots += (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits') }
    foreach ($kitRoot in $kitRoots) {
        if (Test-Path -LiteralPath $kitRoot -PathType Container) {
            $ucrt = Get-ChildItem -Path $kitRoot -Recurse -Filter 'ucrt.lib' -File -ErrorAction SilentlyContinue | Select-Object -First 1
            if ($ucrt) { $sdkFound = $true; break }
        }
    }
    if (-not $sdkFound) {
        $fallbackUcrt = Get-ChildItem -Path 'C:\' -Recurse -Filter 'ucrt.lib' -File -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($fallbackUcrt) { $sdkFound = $true }
    }
    return $sdkFound
}

function Get-StockWatchVisualStudioInstallation {
    $candidates = New-Object System.Collections.Generic.List[string]

    # 1. Standard Visual Studio / Build Tools discovery.
    $vswhereCandidates = @(
        (Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'),
        (Join-Path $env:ProgramFiles 'Microsoft Visual Studio\Installer\vswhere.exe')
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Leaf) } | Select-Object -Unique
    foreach ($vswhere in $vswhereCandidates) {
        try {
            $installs = & $vswhere -all -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
            foreach ($install in $installs) {
                if ($install -and -not [string]::IsNullOrWhiteSpace($install)) {
                    [void]$candidates.Add($install.Trim())
                }
            }
        } catch {
        }
    }

    # 2. Explicit configured/environment override.
    $overrideVars = @('STOCKWATCH_VISUAL_STUDIO_PATH','STOCKWATCH_VISUAL_STUDIO_INSTALLATION','VISUAL_STUDIO_INSTALL_PATH')
    foreach ($varName in $overrideVars) {
        $value = [Environment]::GetEnvironmentVariable($varName, 'Process')
        if (-not $value) { $value = [Environment]::GetEnvironmentVariable($varName, 'User') }
        if (-not $value) { $value = [Environment]::GetEnvironmentVariable($varName, 'Machine') }
        if ($value) { [void]$candidates.Add($value.Trim()) }
    }
    $derivedVars = @('VCINSTALLDIR','VSINSTALLDIR','VCToolsInstallDir')
    foreach ($varName in $derivedVars) {
        $value = [Environment]::GetEnvironmentVariable($varName, 'Process')
        if (-not $value) { $value = [Environment]::GetEnvironmentVariable($varName, 'User') }
        if (-not $value) { $value = [Environment]::GetEnvironmentVariable($varName, 'Machine') }
        if ($value) {
            $trimmed = $value.Trim().TrimEnd('\')
            [void]$candidates.Add($trimmed)
            $parent = Split-Path -Path $trimmed -Parent
            if ($parent) { [void]$candidates.Add($parent) }
        }
    }

    # 3. Safe fallback discovery for custom Build Tools installations.
    $searchRoots = New-Object System.Collections.Generic.List[string]
    if ($env:STOCKWATCH_VISUAL_STUDIO_SEARCH_ROOTS) {
        foreach ($entry in $env:STOCKWATCH_VISUAL_STUDIO_SEARCH_ROOTS.Split(';')) {
            if ($entry) { [void]$searchRoots.Add($entry.Trim()) }
        }
    }
    foreach ($root in @($env:ProgramFiles, ${env:ProgramFiles(x86)}, $env:ProgramW6432)) {
        if ($root -and (Test-Path -LiteralPath $root -PathType Container)) {
            [void]$searchRoots.Add($root)
        }
    }
    $fixedDrives = Get-PSDrive -PSProvider FileSystem -ErrorAction SilentlyContinue | Where-Object { $_.Root -and (Test-Path -LiteralPath $_.Root -PathType Container) }
    foreach ($drive in $fixedDrives) {
        foreach ($sub in @('toolchains','tools','VisualStudio','BuildTools','Microsoft Visual Studio')) {
            $path = Join-Path $drive.Root $sub
            if (Test-Path -LiteralPath $path -PathType Container) {
                [void]$searchRoots.Add($path)
            }
        }
        $depthOne = Get-ChildItem -Path $drive.Root -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -match 'toolchain|visual studio|build tools|buildtools' }
        foreach ($dir in $depthOne) { [void]$searchRoots.Add($dir.FullName) }
    }
    $searchRoots = $searchRoots | Select-Object -Unique
    foreach ($root in $searchRoots) {
        if (-not (Test-Path -LiteralPath $root -PathType Container)) { continue }
        try {
            $found = Get-ChildItem -Path $root -Filter 'VsDevCmd.bat' -Recurse -File -ErrorAction SilentlyContinue -Depth 5
            foreach ($file in $found) {
                $install = Split-Path -Path (Split-Path -Path $file.FullName -Parent) -Parent
                [void]$candidates.Add($install)
                [void]$candidates.Add((Split-Path -Path $install -Parent))
            }
        } catch {
        }
    }

    foreach ($candidate in ($candidates | Select-Object -Unique)) {
        if (Test-StockWatchVisualStudioInstallation -InstallationPath $candidate) {
            return $candidate
        }
    }
    return $null
}

function Initialize-StockWatchVisualStudioEnvironment {
    $installation = Get-StockWatchVisualStudioInstallation
    if (-not $installation) {
        return $false
    }
    $vcvars64 = Join-Path $installation 'VC\Auxiliary\Build\vcvars64.bat'
    $vcvarsAll = Join-Path $installation 'VC\Auxiliary\Build\vcvarsall.bat'
    if (Test-Path -LiteralPath $vcvars64 -PathType Leaf) {
        $vsDevCmd = $vcvars64
        $vsDevArgs = 'x64'
    } elseif (Test-Path -LiteralPath $vcvarsAll -PathType Leaf) {
        $vsDevCmd = $vcvarsAll
        $vsDevArgs = 'x64'
    } else {
        return $false
    }
    $command = 'call "' + $vsDevCmd + '" ' + $vsDevArgs + ' >nul 2>nul && set'
    $lines = & cmd.exe /c $command 2>$null
    foreach ($line in $lines) {
        if ($line -match '^([^=]+)=(.*)$') {
            [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
        }
    }
    return $true
}

function Get-StockWatchMainWindowHandle {
    param([System.Diagnostics.Process]$Process)
    if (-not $Process) { return [IntPtr]::Zero }
    $Process.Refresh()
    $source = @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class StockWatchWindowFinder {
    delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc ep, IntPtr lp);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int n);
    public static IntPtr Find(int pid) {
        IntPtr found=IntPtr.Zero;
        EnumProc cb=(h,l)=>{
            uint wpid;
            GetWindowThreadProcessId(h,out wpid);
            if(wpid==(uint)pid && IsWindowVisible(h)) {
                StringBuilder c=new StringBuilder(256);
                GetClassName(h,c,256);
                string cls=c.ToString();
                bool safe = cls=="Tao Thread Event Target" || cls=="IME" || cls=="Default IME" || cls=="PseudoConsoleWindow";
                if(!safe) { found=h; return false; }
            }
            return true;
        };
        EnumWindows(cb,IntPtr.Zero);
        return found;
    }
}
'@
    try { Add-Type -TypeDefinition $source -ErrorAction Stop } catch { }
    return [StockWatchWindowFinder]::Find([int]$Process.Id)
}
