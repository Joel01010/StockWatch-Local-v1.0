mod repository;

pub use repository::{
    AlertEventRecord, HistoryPruneReport, InsertReport, PortfolioRepository, ProviderStateRecord,
    StorageError,
};

pub const MIGRATION_0001: &str = include_str!("../../migrations/0001_initial.sql");
pub const MIGRATION_0002: &str = include_str!("../../migrations/0002_portfolio_core.sql");
pub const MIGRATION_0003: &str = include_str!("../../migrations/0003_market_data.sql");
pub const MIGRATION_0004: &str = include_str!("../../migrations/0004_alerts_settings.sql");
pub const MIGRATION_0005: &str = include_str!("../../migrations/0005_analytics.sql");
pub const MIGRATION_0006: &str = include_str!("../../migrations/0006_reliability.sql");
pub const SCHEMA_VERSION: i64 = 6;
