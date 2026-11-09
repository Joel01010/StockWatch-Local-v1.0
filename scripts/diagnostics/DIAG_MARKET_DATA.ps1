param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-MARKET'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'market-data.txt';"=== MARKET DATA ==="|Set-Content $out -Encoding UTF8
# Generic liquid NSE symbols are used deliberately; diagnostics must never expose a user's portfolio.
$symbols=@('RELIANCE.NS','TCS.NS','INFY.NS','HDFCBANK.NS')
foreach($s in $symbols){
 $url="https://query1.finance.yahoo.com/v8/finance/chart/${s}?range=5d&interval=1d"
 try{$r=Invoke-RestMethod -Uri $url -Method Get -TimeoutSec 20; $res=$r.chart.result|Select-Object -First 1;if($res){"OK $s currency=$($res.meta.currency) exchange=$($res.meta.exchangeName) regularPrice=$($res.meta.regularMarketPrice) timezone=$($res.meta.exchangeTimezoneName)"|Add-Content $out}else{"NO_RESULT $s"|Add-Content $out}}catch{"FAIL $s :: $($_.Exception.Message)"|Add-Content $out}
}
Get-Content $out
