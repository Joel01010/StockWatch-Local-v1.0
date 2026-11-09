param([string]$OutputDir='')
$ErrorActionPreference='Continue'
. (Join-Path (Split-Path $PSScriptRoot -Parent) 'lib\TestHarness.Common.ps1')
if(-not $OutputDir){$OutputDir=New-TestSession -Prefix 'DIAG-MARKET'};New-Item -ItemType Directory -Force -Path $OutputDir|Out-Null
$out=Join-Path $OutputDir 'market-data.txt';"=== MARKET DATA ==="|Set-Content $out -Encoding UTF8

# Public fallback reachability uses generic liquid NSE symbols; never user holdings.
$symbols=@('RELIANCE.NS','TCS.NS','INFY.NS','HDFCBANK.NS')
foreach($s in $symbols){
 $url="https://query1.finance.yahoo.com/v8/finance/chart/${s}?range=5d&interval=1d"
 try{$r=Invoke-RestMethod -Uri $url -Method Get -TimeoutSec 20; $res=$r.chart.result|Select-Object -First 1;if($res){"YAHOO_OK $s currency=$($res.meta.currency) exchange=$($res.meta.exchangeName) regularPrice=$($res.meta.regularMarketPrice) timezone=$($res.meta.exchangeTimezoneName)"|Add-Content $out}else{"YAHOO_NO_RESULT $s"|Add-Content $out}}catch{"YAHOO_FAIL $s :: $($_.Exception.Message)"|Add-Content $out}
}

# Optional official-provider probe. The credential is read only for the request and is never logged.
$token=[Environment]::GetEnvironmentVariable('STOCKWATCH_UPSTOX_TOKEN')
if([string]::IsNullOrWhiteSpace($token)){
 "UPSTOX_SKIP token not configured"|Add-Content $out
}else{
 try{
  $headers=@{Accept='application/json';Authorization=("Bearer "+$token)}
  $searchUrl='https://api.upstox.com/v2/instruments/search?query=RELIANCE&exchanges=NSE&segments=EQ&page_number=1&records=10'
  $search=Invoke-RestMethod -Uri $searchUrl -Method Get -Headers $headers -TimeoutSec 20
  $instrument=@($search.data | Where-Object { $_.exchange -eq 'NSE' -and $_.trading_symbol -eq 'RELIANCE' -and $_.segment -eq 'NSE_EQ' } | Select-Object -First 1)
  if($instrument.Count -eq 0){throw 'RELIANCE exact NSE_EQ instrument not found'}
  $key=[string]$instrument[0].instrument_key
  $quoteUrl='https://api.upstox.com/v2/market-quote/quotes?instrument_key='+[uri]::EscapeDataString($key)
  $quote=Invoke-RestMethod -Uri $quoteUrl -Method Get -Headers $headers -TimeoutSec 20
  $row=@($quote.data.PSObject.Properties.Value | Select-Object -First 1)
  if($row.Count -eq 0){throw 'quote payload contained no data'}
  "UPSTOX_OK symbol=$($row[0].symbol) lastPrice=$($row[0].last_price) instrument=$key"|Add-Content $out
 }catch{
  "UPSTOX_FAIL $($_.Exception.Message)"|Add-Content $out
 }
}
Get-Content $out
