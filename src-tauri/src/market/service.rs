use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use rust_decimal::prelude::ToPrimitive;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    alerts::AlertService,
    reliability::EventLogger,
    runtime::{RuntimeHolding, RuntimeState},
    scheduler::{estimate_nse_session, BackoffState, RefreshPolicy, WakeDetector},
    settings::{MarketDataSettings, SettingsState},
    storage::PortfolioRepository,
};

use super::{
    FreshnessPolicy, MarketDataError, MarketDataProvider, QuoteFreshness, RefreshReport,
    YahooDelayedProvider,
};

#[derive(Clone)]
pub struct MarketDataService {
    db_path: PathBuf,
    runtime: RuntimeState,
    settings: MarketDataSettings,
    provider: Arc<dyn MarketDataProvider>,
    logger: Option<EventLogger>,
}

impl MarketDataService {
    pub fn new(
        db_path: PathBuf,
        runtime: RuntimeState,
        settings: MarketDataSettings,
    ) -> Result<Self, MarketDataError> {
        let settings = settings.normalized();
        let provider: Arc<dyn MarketDataProvider> = match settings.provider.as_str() {
            "yahoo_delayed" => {
                Arc::new(YahooDelayedProvider::new(settings.request_timeout_seconds)?)
            }
            other => return Err(MarketDataError::ProviderNotConfigured(other.to_string())),
        };
        Ok(Self {
            db_path,
            runtime,
            settings,
            provider,
            logger: None,
        })
    }

    pub fn provider_id(&self) -> &'static str {
        self.provider.provider_id()
    }

    pub fn with_logger(mut self, logger: EventLogger) -> Self {
        self.logger = Some(logger);
        self
    }

    pub async fn refresh_once(&self) -> Result<RefreshReport, MarketDataError> {
        if !self.runtime.try_begin_refresh() {
            return Err(MarketDataError::RefreshAlreadyInFlight);
        }

        let started = Instant::now();
        let outcome = self.refresh_inner().await;
        self.runtime
            .record_refresh_duration(started.elapsed().as_millis().min(u64::MAX as u128) as u64);
        match &outcome {
            Ok(report) => self.runtime.finish_refresh_ok(report.observed_unix_ms),
            Err(error) => {
                self.runtime.finish_refresh_error(error.to_string());
                if let Some(logger) = &self.logger {
                    logger.append("WARN", "market_refresh_failed", &error.to_string());
                }
                if let Ok(repo) = PortfolioRepository::open(&self.db_path) {
                    let _ =
                        repo.record_provider_failure(self.provider_id(), &error.to_string(), None);
                    let _ = self.sync_runtime_from_cache(&repo, now_unix_ms());
                }
            }
        }
        outcome
    }

    async fn refresh_inner(&self) -> Result<RefreshReport, MarketDataError> {
        let targets = {
            let repo = PortfolioRepository::open(&self.db_path)
                .map_err(|error| MarketDataError::Storage(error.to_string()))?;
            repo.quote_targets(self.provider_id())
                .map_err(|error| MarketDataError::Storage(error.to_string()))?
        };

        let now = OffsetDateTime::now_utc();
        let observed_unix_ms = unix_ms(now);
        let session = estimate_nse_session(now);
        self.runtime.update(|state| {
            state.market_session = session;
            state.provider_id = Some(self.provider_id().to_string());
        });

        if targets.is_empty() {
            let repo = PortfolioRepository::open(&self.db_path)
                .map_err(|error| MarketDataError::Storage(error.to_string()))?;
            self.sync_runtime_from_cache(&repo, observed_unix_ms)?;
            return Ok(RefreshReport {
                provider_id: self.provider_id().into(),
                requested: 0,
                received: 0,
                persisted: 0,
                failed_symbols: Vec::new(),
                observed_unix_ms,
            });
        }

        let quotes = self.provider.quotes(&targets).await?;
        let mut repo = PortfolioRepository::open(&self.db_path)
            .map_err(|error| MarketDataError::Storage(error.to_string()))?;
        let persisted = repo
            .upsert_quotes_atomic(&quotes, self.settings.history_enabled)
            .map_err(|error| MarketDataError::Storage(error.to_string()))?;
        repo.record_provider_success(
            self.provider_id(),
            &now.format(&Rfc3339).unwrap_or_default(),
        )
        .map_err(|error| MarketDataError::Storage(error.to_string()))?;
        self.sync_runtime_from_cache(&repo, observed_unix_ms)?;
        // Analytics snapshots are secondary to quote persistence. A snapshot failure must not
        // turn a successful market refresh into a provider failure, but fully priced portfolios
        // are captured in compact five-minute buckets whenever possible.
        let _ = self.record_performance_snapshot(&mut repo, observed_unix_ms);

        let received_keys: std::collections::HashSet<String> = quotes
            .iter()
            .map(|q| format!("{}:{}", q.exchange, q.symbol))
            .collect();
        let failed_symbols = targets
            .iter()
            .filter(|target| !received_keys.contains(&target.key()))
            .map(|target| target.key())
            .collect();

        Ok(RefreshReport {
            provider_id: self.provider_id().into(),
            requested: targets.len(),
            received: quotes.len(),
            persisted,
            failed_symbols,
            observed_unix_ms,
        })
    }

    fn record_performance_snapshot(
        &self,
        repo: &mut PortfolioRepository,
        observed_unix_ms: i64,
    ) -> Result<(), MarketDataError> {
        let portfolio = repo
            .derive_portfolio()
            .map_err(|error| MarketDataError::Storage(error.to_string()))?;
        let runtime = self.runtime.snapshot();
        let mut holding_values = Vec::new();
        let mut portfolio_value = 0.0;
        let mut open_count = 0usize;
        for holding in portfolio.open_holdings() {
            open_count += 1;
            let quantity = holding.quantity.to_f64().unwrap_or(0.0);
            let Some(row) = runtime.holdings.iter().find(|row| {
                row.exchange
                    .eq_ignore_ascii_case(&holding.instrument.exchange)
                    && row.symbol.eq_ignore_ascii_case(&holding.instrument.symbol)
            }) else {
                return Ok(());
            };
            if !matches!(
                row.freshness,
                QuoteFreshness::Live | QuoteFreshness::Fresh | QuoteFreshness::Delayed
            ) {
                return Ok(());
            }
            let Some(price) = row.price.filter(|value| value.is_finite() && *value >= 0.0) else {
                return Ok(());
            };
            let current_value = price * quantity;
            portfolio_value += current_value;
            holding_values.push((
                holding.instrument.exchange.clone(),
                holding.instrument.symbol.clone(),
                price,
                current_value,
            ));
        }
        if open_count == 0 || holding_values.len() != open_count {
            return Ok(());
        }
        repo.upsert_performance_snapshot(
            observed_unix_ms,
            portfolio_value,
            &holding_values,
            self.provider_id(),
        )
        .map_err(|error| MarketDataError::Storage(error.to_string()))?;
        Ok(())
    }

    pub fn refresh_runtime_from_cache(&self) -> Result<(), MarketDataError> {
        let repo = PortfolioRepository::open(&self.db_path)
            .map_err(|error| MarketDataError::Storage(error.to_string()))?;
        self.sync_runtime_from_cache(&repo, now_unix_ms())
    }

    fn sync_runtime_from_cache(
        &self,
        repo: &PortfolioRepository,
        now_ms: i64,
    ) -> Result<(), MarketDataError> {
        let cached = repo
            .latest_quotes()
            .map_err(|error| MarketDataError::Storage(error.to_string()))?;
        let portfolio = repo
            .derive_portfolio()
            .map_err(|error| MarketDataError::Storage(error.to_string()))?;
        let policy = FreshnessPolicy {
            stale_after_seconds: self.settings.stale_after_seconds,
            offline_after_seconds: self.settings.offline_after_seconds,
        };

        let mut by_key = HashMap::new();
        let holdings: Vec<RuntimeHolding> = cached
            .into_iter()
            .map(|quote| {
                let age_seconds =
                    now_ms.saturating_sub(quote.observed_unix_ms).max(0) as u64 / 1000;
                let freshness = policy.classify(quote.freshness, age_seconds);
                by_key.insert(
                    format!("{}:{}", quote.exchange, quote.symbol),
                    (quote.price, quote.change_pct, freshness),
                );
                RuntimeHolding {
                    symbol: quote.symbol,
                    exchange: quote.exchange,
                    price: Some(quote.price),
                    change_pct: quote.change_pct,
                    freshness,
                    source: Some(quote.source),
                    observed_unix_ms: Some(quote.observed_unix_ms),
                }
            })
            .collect();

        let mut portfolio_value = 0.0;
        let mut invested = 0.0;
        let mut day_pnl = 0.0;
        let mut all_priced = true;
        let mut any_day = false;
        let mut open_count = 0usize;
        for holding in portfolio.open_holdings() {
            open_count += 1;
            let key = format!(
                "{}:{}",
                holding.instrument.exchange, holding.instrument.symbol
            );
            let quantity = holding.quantity.to_f64().unwrap_or(0.0);
            let cost = holding.gross_cost_basis().to_f64().unwrap_or(0.0);
            invested += cost;
            match by_key.get(&key) {
                Some((price, change_pct, _)) => {
                    portfolio_value += price * quantity;
                    if let Some(change) = change_pct {
                        let divisor = 1.0 + change / 100.0;
                        if divisor.abs() > f64::EPSILON {
                            let prior = price / divisor;
                            day_pnl += (price - prior) * quantity;
                            any_day = true;
                        }
                    }
                }
                None => all_priced = false,
            }
        }
        let portfolio_value = if all_priced && open_count > 0 {
            Some(portfolio_value)
        } else {
            None
        };
        let total_pnl = portfolio_value.map(|value| value - invested);
        let day_pnl = if any_day { Some(day_pnl) } else { None };

        self.runtime.update(move |state| {
            state.holdings = holdings;
            state.portfolio_value = portfolio_value;
            state.total_pnl = total_pnl;
            state.day_pnl = day_pnl;
        });
        Ok(())
    }
}

pub fn spawn_market_worker<R: tauri::Runtime>(
    service: MarketDataService,
    settings: SettingsState,
    alerts: AlertService,
    app: tauri::AppHandle<R>,
    on_runtime_change: Arc<dyn Fn() + Send + Sync>,
) {
    tauri::async_runtime::spawn(async move {
        let mut backoff = BackoffState::default();
        let mut wake = WakeDetector::new(180);
        loop {
            let now = OffsetDateTime::now_utc();
            let now_ms = now.unix_timestamp_nanos().saturating_div(1_000_000) as i64;
            if wake.observe(now_ms) {
                backoff.success();
                service.runtime.note_wake_recovery();
                let _ = service.refresh_runtime_from_cache();
            }
            let session = estimate_nse_session(now);
            service
                .runtime
                .update(|state| state.market_session = session);
            let _ = service.refresh_runtime_from_cache();
            on_runtime_change();

            let policy: RefreshPolicy = settings.snapshot().refresh;
            let interval = policy.interval(session);
            if interval.is_none() {
                // With polling intentionally stopped (normally market closed), cache aging can
                // still drive an explicitly non-market-hours-only feed-health rule.
                let _ = alerts.evaluate_all(&app);
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }

            match service.refresh_once().await {
                Ok(_) => {
                    backoff.success();
                    let _ = alerts.evaluate_all(&app);
                    on_runtime_change();
                    tokio::time::sleep(interval.unwrap_or(Duration::from_secs(60))).await;
                }
                Err(MarketDataError::RefreshAlreadyInFlight) => {
                    on_runtime_change();
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Err(_) => {
                    backoff.failure();
                    let _ = alerts.evaluate_all(&app);
                    on_runtime_change();
                    tokio::time::sleep(backoff.delay(5, policy.failure_backoff_max_seconds)).await;
                }
            }
        }
    });
}

fn now_unix_ms() -> i64 {
    unix_ms(OffsetDateTime::now_utc())
}
fn unix_ms(value: OffsetDateTime) -> i64 {
    value.unix_timestamp_nanos().saturating_div(1_000_000) as i64
}
