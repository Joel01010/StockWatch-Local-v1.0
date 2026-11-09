use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuoteFreshness {
    Live,
    Fresh,
    Delayed,
    Stale,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MarketInstrument {
    pub exchange: String,
    pub symbol: String,
    pub name: Option<String>,
    pub provider_symbol: Option<String>,
}

impl MarketInstrument {
    pub fn normalized(
        exchange: impl Into<String>,
        symbol: impl Into<String>,
        name: Option<String>,
    ) -> Self {
        Self {
            exchange: exchange.into().trim().to_ascii_uppercase(),
            symbol: symbol.into().trim().to_ascii_uppercase(),
            name,
            provider_symbol: None,
        }
    }

    pub fn with_provider_symbol(mut self, provider_symbol: Option<String>) -> Self {
        self.provider_symbol = provider_symbol
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        self
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.exchange, self.symbol)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Quote {
    pub symbol: String,
    pub exchange: String,
    pub price: f64,
    pub change_pct: Option<f64>,
    pub observed_unix_ms: i64,
    pub source: String,
    pub freshness: QuoteFreshness,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefreshReport {
    pub provider_id: String,
    pub requested: usize,
    pub received: usize,
    pub persisted: usize,
    pub failed_symbols: Vec<String>,
    pub observed_unix_ms: i64,
}
