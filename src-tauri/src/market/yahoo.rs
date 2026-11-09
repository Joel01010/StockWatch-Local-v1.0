use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use super::{MarketDataError, MarketDataProvider, MarketInstrument, Quote, QuoteFreshness};

#[derive(Clone)]
pub struct YahooDelayedProvider {
    client: Client,
}

impl YahooDelayedProvider {
    pub fn new(timeout_seconds: u64) -> Result<Self, MarketDataError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds.clamp(2, 30)))
            .user_agent("StockWatchLocal/0.4")
            .build()
            .map_err(|error| MarketDataError::Unavailable(error.to_string()))?;
        Ok(Self { client })
    }

    pub fn provider_symbol(instrument: &MarketInstrument) -> Result<String, MarketDataError> {
        if let Some(provider_symbol) = &instrument.provider_symbol {
            return Ok(provider_symbol.clone());
        }
        if instrument.symbol.contains('.') {
            return Ok(instrument.symbol.clone());
        }
        match instrument.exchange.as_str() {
            "NSE" => Ok(format!("{}.NS", instrument.symbol)),
            "BSE" => Ok(format!("{}.BO", instrument.symbol)),
            exchange => Err(MarketDataError::UnsupportedExchange(exchange.to_string())),
        }
    }

    async fn one_quote(&self, instrument: &MarketInstrument) -> Result<Quote, MarketDataError> {
        let provider_symbol = Self::provider_symbol(instrument)?;
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1m&range=1d",
            urlencoding::encode(&provider_symbol)
        );
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|error| MarketDataError::Unavailable(error.to_string()))?;
        if !response.status().is_success() {
            return Err(MarketDataError::Unavailable(format!(
                "HTTP {}",
                response.status()
            )));
        }
        let payload: YahooChartResponse = response
            .json()
            .await
            .map_err(|error| MarketDataError::InvalidData(error.to_string()))?;
        let result = payload
            .chart
            .result
            .and_then(|mut rows| rows.pop())
            .ok_or_else(|| MarketDataError::InstrumentNotFound(instrument.key()))?;
        let price = result.meta.regular_market_price.ok_or_else(|| {
            MarketDataError::InvalidData(format!("missing market price for {}", instrument.key()))
        })?;
        if !price.is_finite() || price < 0.0 {
            return Err(MarketDataError::InvalidData(format!(
                "invalid price for {}",
                instrument.key()
            )));
        }
        let previous = result
            .meta
            .chart_previous_close
            .or(result.meta.previous_close);
        let change_pct = previous.and_then(|prior| {
            if prior.abs() <= f64::EPSILON {
                None
            } else {
                Some((price - prior) / prior * 100.0)
            }
        });
        let observed_unix_ms = result
            .meta
            .regular_market_time
            .map(|seconds| seconds.saturating_mul(1000))
            .unwrap_or_else(now_unix_ms);

        Ok(Quote {
            symbol: instrument.symbol.clone(),
            exchange: instrument.exchange.clone(),
            price,
            change_pct,
            observed_unix_ms,
            source: self.provider_id().to_string(),
            freshness: QuoteFreshness::Delayed,
        })
    }
}

#[async_trait]
impl MarketDataProvider for YahooDelayedProvider {
    fn provider_id(&self) -> &'static str {
        "yahoo_delayed"
    }
    fn is_delayed(&self) -> bool {
        true
    }

    async fn quotes(
        &self,
        instruments: &[MarketInstrument],
    ) -> Result<Vec<Quote>, MarketDataError> {
        let mut quotes = Vec::new();
        let mut failures = Vec::new();
        for instrument in instruments {
            match self.one_quote(instrument).await {
                Ok(quote) => quotes.push(quote),
                Err(error) => failures.push(format!("{}: {}", instrument.key(), error)),
            }
        }
        if quotes.is_empty() && !instruments.is_empty() {
            return Err(MarketDataError::Unavailable(failures.join("; ")));
        }
        Ok(quotes)
    }
}

#[derive(Debug, Deserialize)]
struct YahooChartResponse {
    chart: YahooChart,
}
#[derive(Debug, Deserialize)]
struct YahooChart {
    result: Option<Vec<YahooChartResult>>,
}
#[derive(Debug, Deserialize)]
struct YahooChartResult {
    meta: YahooMeta,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct YahooMeta {
    regular_market_price: Option<f64>,
    regular_market_time: Option<i64>,
    chart_previous_close: Option<f64>,
    previous_close: Option<f64>,
}

fn now_unix_ms() -> i64 {
    time::OffsetDateTime::now_utc()
        .unix_timestamp_nanos()
        .saturating_div(1_000_000) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_nse_and_bse_symbols() {
        assert_eq!(
            YahooDelayedProvider::provider_symbol(&MarketInstrument::normalized(
                "NSE", "DEMOTECH", None
            ))
            .unwrap(),
            "DEMOTECH.NS"
        );
        assert_eq!(
            YahooDelayedProvider::provider_symbol(&MarketInstrument::normalized(
                "BSE", "500300", None
            ))
            .unwrap(),
            "500300.BO"
        );
    }

    #[test]
    fn preserves_explicit_provider_suffix() {
        assert_eq!(
            YahooDelayedProvider::provider_symbol(&MarketInstrument::normalized(
                "NSE",
                "DEMOETF.NS",
                None
            ))
            .unwrap(),
            "DEMOETF.NS"
        );
    }
}
