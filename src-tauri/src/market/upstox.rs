use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use reqwest::{header::ACCEPT, Client};
use serde::Deserialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::{MarketDataError, MarketDataProvider, MarketInstrument, Quote, QuoteFreshness};

pub const UPSTOX_TOKEN_ENV: &str = "STOCKWATCH_UPSTOX_TOKEN";

#[derive(Clone)]
pub struct UpstoxAnalyticsProvider {
    client: Client,
    token: Arc<String>,
    resolved_keys: Arc<Mutex<HashMap<String, String>>>,
}

impl UpstoxAnalyticsProvider {
    pub fn token_configured() -> bool {
        std::env::var(UPSTOX_TOKEN_ENV)
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn from_env(timeout_seconds: u64) -> Result<Self, MarketDataError> {
        let token = std::env::var(UPSTOX_TOKEN_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                MarketDataError::ProviderNotConfigured(format!(
                    "Upstox Analytics Token missing; set {UPSTOX_TOKEN_ENV}"
                ))
            })?;
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds.clamp(2, 30)))
            .user_agent("StockWatchLocal/1.0")
            .build()
            .map_err(|error| MarketDataError::Unavailable(error.to_string()))?;
        Ok(Self {
            client,
            token: Arc::new(token),
            resolved_keys: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    async fn resolve_instrument_key(
        &self,
        instrument: &MarketInstrument,
    ) -> Result<String, MarketDataError> {
        if let Some(override_key) = instrument
            .provider_symbol
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            return Ok(override_key.to_string());
        }

        let cache_key = instrument.key();
        if let Ok(cache) = self.resolved_keys.lock() {
            if let Some(value) = cache.get(&cache_key) {
                return Ok(value.clone());
            }
        }

        let response = self
            .client
            .get("https://api.upstox.com/v2/instruments/search")
            .header(ACCEPT, "application/json")
            .bearer_auth(self.token.as_str())
            .query(&[
                ("query", instrument.symbol.as_str()),
                ("exchanges", instrument.exchange.as_str()),
                ("segments", "EQ"),
                ("page_number", "1"),
                ("records", "30"),
            ])
            .send()
            .await
            .map_err(|error| MarketDataError::Unavailable(error.to_string()))?;

        if !response.status().is_success() {
            return Err(MarketDataError::Unavailable(format!(
                "Upstox instrument search HTTP {}",
                response.status()
            )));
        }

        let payload: InstrumentSearchResponse = response
            .json()
            .await
            .map_err(|error| MarketDataError::InvalidData(error.to_string()))?;

        let expected_segment = format!("{}_EQ", instrument.exchange);
        let exact = payload.data.into_iter().find(|row| {
            row.exchange.eq_ignore_ascii_case(&instrument.exchange)
                && row.trading_symbol.eq_ignore_ascii_case(&instrument.symbol)
                && row.segment.eq_ignore_ascii_case(&expected_segment)
        });

        let key = exact
            .map(|row| row.instrument_key)
            .ok_or_else(|| MarketDataError::InstrumentNotFound(instrument.key()))?;

        if let Ok(mut cache) = self.resolved_keys.lock() {
            cache.insert(cache_key, key.clone());
        }
        Ok(key)
    }

    async fn full_quotes(
        &self,
        instrument_keys: &[String],
    ) -> Result<FullQuoteResponse, MarketDataError> {
        let joined = instrument_keys.join(",");
        let response = self
            .client
            .get("https://api.upstox.com/v2/market-quote/quotes")
            .header(ACCEPT, "application/json")
            .bearer_auth(self.token.as_str())
            .query(&[("instrument_key", joined.as_str())])
            .send()
            .await
            .map_err(|error| MarketDataError::Unavailable(error.to_string()))?;

        if !response.status().is_success() {
            return Err(MarketDataError::Unavailable(format!(
                "Upstox market quote HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| MarketDataError::InvalidData(error.to_string()))
    }
}

#[async_trait]
impl MarketDataProvider for UpstoxAnalyticsProvider {
    fn provider_id(&self) -> &'static str {
        "upstox_analytics"
    }

    fn is_delayed(&self) -> bool {
        false
    }

    async fn quotes(
        &self,
        instruments: &[MarketInstrument],
    ) -> Result<Vec<Quote>, MarketDataError> {
        if instruments.is_empty() {
            return Ok(Vec::new());
        }

        let mut resolved = Vec::new();
        let mut failures = Vec::new();
        for instrument in instruments {
            match self.resolve_instrument_key(instrument).await {
                Ok(key) => resolved.push((instrument.clone(), key)),
                Err(error) => failures.push(format!("{}: {error}", instrument.key())),
            }
        }

        if resolved.is_empty() {
            return Err(MarketDataError::Unavailable(failures.join("; ")));
        }

        let keys: Vec<String> = resolved.iter().map(|(_, key)| key.clone()).collect();
        let payload = self.full_quotes(&keys).await?;
        let mut quotes = Vec::new();

        for (instrument, key) in resolved {
            let row = payload.data.values().find(|row| {
                row.instrument_token == key
                    || row.symbol.eq_ignore_ascii_case(&instrument.symbol)
            });
            let Some(row) = row else {
                continue;
            };
            if !row.last_price.is_finite() || row.last_price < 0.0 {
                continue;
            }

            let previous_close = row
                .net_change
                .map(|change| row.last_price - change)
                .filter(|value| value.abs() > f64::EPSILON)
                .or_else(|| {
                    row.ohlc
                        .as_ref()
                        .map(|ohlc| ohlc.close)
                        .filter(|value| value.abs() > f64::EPSILON)
                });
            let change_pct = previous_close.map(|prior| (row.last_price - prior) / prior * 100.0);
            let observed_unix_ms = parse_observed_unix_ms(row).unwrap_or_else(now_unix_ms);

            quotes.push(Quote {
                symbol: instrument.symbol,
                exchange: instrument.exchange,
                price: row.last_price,
                change_pct,
                observed_unix_ms,
                source: self.provider_id().to_string(),
                // REST gives an exchange-backed snapshot. Reserve "live" for streaming feeds.
                freshness: QuoteFreshness::Fresh,
            });
        }

        if quotes.is_empty() {
            return Err(MarketDataError::Unavailable(
                "Upstox returned no usable quotes".into(),
            ));
        }
        Ok(quotes)
    }
}

fn parse_observed_unix_ms(row: &FullQuoteRow) -> Option<i64> {
    row.last_trade_time
        .as_deref()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .or_else(|| {
            row.timestamp.as_deref().and_then(|value| {
                OffsetDateTime::parse(value, &Rfc3339)
                    .ok()
                    .map(|time| time.unix_timestamp_nanos().saturating_div(1_000_000) as i64)
            })
        })
}

fn now_unix_ms() -> i64 {
    OffsetDateTime::now_utc()
        .unix_timestamp_nanos()
        .saturating_div(1_000_000) as i64
}

#[derive(Debug, Deserialize)]
struct InstrumentSearchResponse {
    #[serde(default)]
    data: Vec<InstrumentSearchRow>,
}

#[derive(Debug, Deserialize)]
struct InstrumentSearchRow {
    segment: String,
    exchange: String,
    instrument_key: String,
    trading_symbol: String,
}

#[derive(Debug, Deserialize)]
struct FullQuoteResponse {
    #[serde(default)]
    data: HashMap<String, FullQuoteRow>,
}

#[derive(Debug, Deserialize)]
struct FullQuoteRow {
    instrument_token: String,
    symbol: String,
    last_price: f64,
    net_change: Option<f64>,
    last_trade_time: Option<String>,
    timestamp: Option<String>,
    ohlc: Option<UpstoxOhlc>,
}

#[derive(Debug, Deserialize)]
struct UpstoxOhlc {
    close: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_documented_full_quote_shape() {
        let payload: FullQuoteResponse = serde_json::from_str(
            r#"{
              "status":"success",
              "data":{
                "NSE_EQ:DEMO":{
                  "ohlc":{"close":100.0},
                  "timestamp":"2026-11-09T10:00:00+05:30",
                  "instrument_token":"NSE_EQ|INE000000001",
                  "symbol":"DEMO",
                  "last_price":101.0,
                  "net_change":1.0,
                  "last_trade_time":"1794198600000"
                }
              }
            }"#,
        )
        .unwrap();
        let row = payload.data.get("NSE_EQ:DEMO").unwrap();
        assert_eq!(row.symbol, "DEMO");
        assert_eq!(row.last_price, 101.0);
        assert_eq!(parse_observed_unix_ms(row), Some(1_794_198_600_000));
    }

    #[test]
    fn token_detection_rejects_blank_values() {
        let name = UPSTOX_TOKEN_ENV;
        let previous = std::env::var(name).ok();
        std::env::set_var(name, "   ");
        assert!(!UpstoxAnalyticsProvider::token_configured());
        if let Some(value) = previous {
            std::env::set_var(name, value);
        } else {
            std::env::remove_var(name);
        }
    }
}
