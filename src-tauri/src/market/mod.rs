mod freshness;
mod model;
mod provider;
mod service;
mod yahoo;

pub use freshness::FreshnessPolicy;
pub use model::{MarketInstrument, Quote, QuoteFreshness, RefreshReport};
pub use provider::MarketDataProvider;
pub use service::{spawn_market_worker, MarketDataService};
pub use yahoo::YahooDelayedProvider;

#[derive(Debug, thiserror::Error)]
pub enum MarketDataError {
    #[error("provider unavailable: {0}")]
    Unavailable(String),
    #[error("instrument not found: {0}")]
    InstrumentNotFound(String),
    #[error("provider returned invalid data: {0}")]
    InvalidData(String),
    #[error("unsupported exchange: {0}")]
    UnsupportedExchange(String),
    #[error("market provider is not configured: {0}")]
    ProviderNotConfigured(String),
    #[error("refresh already in flight")]
    RefreshAlreadyInFlight,
    #[error("storage error: {0}")]
    Storage(String),
}
