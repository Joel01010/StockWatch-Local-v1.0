use async_trait::async_trait;

use super::{MarketDataError, MarketInstrument, Quote};

#[async_trait]
pub trait MarketDataProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn is_delayed(&self) -> bool;
    async fn quotes(&self, instruments: &[MarketInstrument])
        -> Result<Vec<Quote>, MarketDataError>;
}
