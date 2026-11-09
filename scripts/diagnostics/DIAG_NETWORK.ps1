param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-NETWORK'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'network.txt';"=== NETWORK / DNS / TLS ==="|Set-Content $out -Encoding UTF8
"--- WinHTTP proxy ---"|Add-Content $out; netsh winhttp show proxy 2>&1|Add-Content $out
"--- Proxy env names only ---"|Add-Content $out
foreach($n in @('HTTP_PROXY','HTTPS_PROXY','ALL_PROXY','NO_PROXY','NPM_CONFIG_PROXY','NPM_CONFIG_HTTPS_PROXY')){$v=[Environment]::GetEnvironmentVariable($n);if($v){try{$u=[uri]$v;"$n=set host=$($u.Host) port=$($u.Port)"|Add-Content $out}catch{"$n=set (value redacted)"|Add-Content $out}}else{"$n=unset"|Add-Content $out}}
$hosts=@('registry.npmjs.org','crates.io','static.crates.io','github.com','query1.finance.yahoo.com')
foreach($h in $hosts){
 "--- $h ---"|Add-Content $out
 try{Resolve-DnsName $h -ErrorAction Stop|Select-Object Name,Type,IPAddress|Format-Table|Out-String|Add-Content $out}catch{"DNS_FAIL $($_.Exception.Message)"|Add-Content $out}
 try{$t=Test-NetConnection $h -Port 443 -WarningAction SilentlyContinue;"TCP443=$($t.TcpTestSucceeded) Remote=$($t.RemoteAddress)"|Add-Content $out}catch{"TCP_FAIL $($_.Exception.Message)"|Add-Content $out}
}
foreach($url in @('https://registry.npmjs.org/','https://crates.io/','https://query1.finance.yahoo.com/v8/finance/chart/RELIANCE.NS?range=1d&interval=1d')){
 try{$r=Invoke-WebRequest -Uri $url -Method Get -TimeoutSec 15 -MaximumRedirection 3 -UseBasicParsing;"HTTP_OK $url status=$($r.StatusCode) bytes=$($r.RawContentLength)"|Add-Content $out}catch{"HTTP_FAIL $url :: $($_.Exception.Message)"|Add-Content $out}
}
Get-Content $out
