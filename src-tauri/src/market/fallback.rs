use std::{collections::HashSet, sync::Arc};

use async_trait::async_trait;

use super::{MarketDataError, MarketDataProvider, MarketInstrument, Quote};

pub struct FallbackMarketDataProvider {
    primary: Arc<dyn MarketDataProvider>,
    fallback: Arc<dyn MarketDataProvider>,
}

impl FallbackMarketDataProvider {
    pub fn new(
        primary: Arc<dyn MarketDataProvider>,
        fallback: Arc<dyn MarketDataProvider>,
    ) -> Self {
        Self { primary, fallback }
    }
}

#[async_trait]
impl MarketDataProvider for FallbackMarketDataProvider {
    fn provider_id(&self) -> &'static str {
        "upstox_yahoo_fallback"
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

        let mut primary_error = None;
        let mut quotes = match self.primary.quotes(instruments).await {
            Ok(rows) => rows,
            Err(error) => {
                primary_error = Some(error.to_string());
                Vec::new()
            }
        };

        let received: HashSet<String> = quotes
            .iter()
            .map(|quote| format!("{}:{}", quote.exchange, quote.symbol))
            .collect();
        let missing: Vec<MarketInstrument> = instruments
            .iter()
            .filter(|instrument| !received.contains(&instrument.key()))
            .cloned()
            .collect();

        if missing.is_empty() {
            return Ok(quotes);
        }

        match self.fallback.quotes(&missing).await {
            Ok(mut rows) => {
                quotes.append(&mut rows);
                Ok(quotes)
            }
            Err(fallback_error) if quotes.is_empty() => Err(MarketDataError::Unavailable(format!(
                "primary failed: {}; fallback failed: {}",
                primary_error.unwrap_or_else(|| "no usable primary quotes".into()),
                fallback_error
            ))),
            Err(_) => Ok(quotes),
        }
    }
}
