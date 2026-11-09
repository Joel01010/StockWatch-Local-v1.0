param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-TOOLCHAIN'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'toolchain.txt'
"=== TOOLCHAIN ==="|Set-Content $out -Encoding UTF8
"PowerShell=$($PSVersionTable.PSVersion)"|Add-Content $out
"OS=$([Environment]::OSVersion.VersionString)"|Add-Content $out
"PROCESS_ARCH=$env:PROCESSOR_ARCHITECTURE"|Add-Content $out
foreach($t in @('node','npm','npx','cargo','rustc','rustup','python','python3','git','cl.exe','link.exe')){
 $c=Get-Command $t -ErrorAction SilentlyContinue
 if($c){"FOUND $t = $($c.Source)"|Add-Content $out;try{(& $t --version 2>&1|Select-Object -First 3)|ForEach-Object{"  $_"}|Add-Content $out}catch{}}
 else{"MISSING $t"|Add-Content $out}
}
$vswhere=@("$env:ProgramFiles(x86)\Microsoft Visual Studio\Installer\vswhere.exe","$env:ProgramFiles\Microsoft Visual Studio\Installer\vswhere.exe")|Where-Object{Test-Path $_}|Select-Object -First 1
if($vswhere){"VSWHERE=$vswhere"|Add-Content $out; & $vswhere -all -products * -format json 2>&1 | Add-Content $out}else{"VSWHERE missing"|Add-Content $out}
"--- Windows SDK roots ---"|Add-Content $out
@("$env:ProgramFiles(x86)\Windows Kits\10","$env:ProgramFiles\Windows Kits\10")|ForEach-Object{if(Test-Path $_){Get-ChildItem $_ -Depth 2 -ErrorAction SilentlyContinue|Select-Object -First 100 FullName|Format-Table -HideTableHeaders|Out-String|Add-Content $out}}
"--- PATH entries ---"|Add-Content $out
($env:PATH -split ';')|ForEach-Object{"  $_"}|Add-Content $out
Get-Content $out
