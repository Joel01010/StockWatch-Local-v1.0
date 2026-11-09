use std::path::PathBuf;

use rust_decimal::prelude::ToPrimitive;
use tauri::{AppHandle, Runtime};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

use super::{evaluate, event_text, is_quiet_hours, AlertEvent, AlertMetricContext, AlertRule};
use crate::{
    notifications::send_native,
    portfolio::Holding,
    runtime::RuntimeState,
    settings::SettingsState,
    storage::{AlertEventRecord, PortfolioRepository},
};

#[derive(Clone)]
pub struct AlertService {
    db_path: PathBuf,
    runtime: RuntimeState,
    settings: SettingsState,
}

impl AlertService {
    pub fn new(db_path: PathBuf, runtime: RuntimeState, settings: SettingsState) -> Self {
        Self {
            db_path,
            runtime,
            settings,
        }
    }

    pub fn list_rules(&self) -> Result<Vec<AlertRule>, String> {
        PortfolioRepository::open(&self.db_path)
            .map_err(|e| e.to_string())?
            .list_alert_rules()
            .map_err(|e| e.to_string())
    }
    pub fn save_rule(&self, rule: AlertRule) -> Result<AlertRule, String> {
        let mut rule = rule.normalized();
        if rule.id.is_empty() {
            rule.id = format!("alert-{}", now_unix_ms());
        }
        if rule.created_at.is_none() {
            rule.created_at = Some(
                OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .unwrap_or_default(),
            );
        }
        validate_rule(&rule)?;
        let repo = PortfolioRepository::open(&self.db_path).map_err(|e| e.to_string())?;

        if requires_open_holding(&rule.condition) {
            let portfolio = repo.derive_portfolio().map_err(|e| e.to_string())?;
            let found = portfolio.open_holdings().any(|holding| {
                rule.symbol
                    .as_ref()
                    .map(|symbol| holding.instrument.symbol.eq_ignore_ascii_case(symbol))
                    .unwrap_or(false)
                    && rule
                        .exchange
                        .as_ref()
                        .map(|exchange| holding.instrument.exchange.eq_ignore_ascii_case(exchange))
                        .unwrap_or(false)
            });
            if !found {
                return Err("holding return/P&L alerts require an open portfolio holding".into());
            }
        }

        if !rule.id.is_empty() && rule.repeat_mode == super::AlertRepeatMode::Once && rule.enabled {
            if let Ok(existing) = repo.list_alert_rules() {
                if existing.iter().any(|old| old.id == rule.id && !old.enabled) {
                    rule.last_triggered_at = None;
                    rule.last_value = None;
                    rule.armed = true;
                }
            }
        }
        repo.upsert_alert_rule(&rule).map_err(|e| e.to_string())?;
        self.refresh_active_count()?;
        Ok(rule)
    }
    pub fn delete_rule(&self, id: &str) -> Result<bool, String> {
        let result = PortfolioRepository::open(&self.db_path)
            .map_err(|e| e.to_string())?
            .delete_alert_rule(id)
            .map_err(|e| e.to_string())?;
        self.refresh_active_count()?;
        Ok(result)
    }
    pub fn recent_events(&self, limit: usize) -> Result<Vec<AlertEventRecord>, String> {
        PortfolioRepository::open(&self.db_path)
            .map_err(|e| e.to_string())?
            .recent_alert_events(limit)
            .map_err(|e| e.to_string())
    }
    pub fn refresh_active_count(&self) -> Result<(), String> {
        let count = self.list_rules()?.iter().filter(|r| r.enabled).count();
        self.runtime.update(|s| s.active_alert_count = count);
        Ok(())
    }

    pub fn evaluate_all<R: Runtime>(&self, app: &AppHandle<R>) -> Result<usize, String> {
        let repo = PortfolioRepository::open(&self.db_path).map_err(|e| e.to_string())?;
        let portfolio = repo.derive_portfolio().map_err(|e| e.to_string())?;
        let runtime = self.runtime.snapshot();
        let settings = self.settings.snapshot();
        let now = OffsetDateTime::now_utc();
        let now_ms = now.unix_timestamp_nanos().saturating_div(1_000_000) as i64;
        let mut rules = repo.list_alert_rules().map_err(|e| e.to_string())?;
        let mut fired = 0usize;
        for rule in &mut rules {
            let ctx = context_for_rule(rule, &portfolio, &runtime, now_ms);
            let result = evaluate(rule, &ctx);
            repo.update_alert_runtime(rule).map_err(|e| e.to_string())?;
            if !result.should_trigger {
                continue;
            }
            fired += 1;
            let metric = result.metric_value.unwrap_or_default();
            let (title, body) = event_text(rule, metric);
            let (status, error) = if !settings.notifications.enabled {
                ("recorded_only".to_string(), None)
            } else if is_quiet_hours(&settings.notifications, now) {
                ("suppressed_quiet".to_string(), None)
            } else {
                match send_native(app, &title, &body, settings.notifications.sound) {
                    Ok(()) => ("sent".into(), None),
                    Err(e) => ("notification_error".into(), Some(e)),
                }
            };
            let event = AlertEvent {
                alert_id: rule.id.clone(),
                fired_at: now.format(&Rfc3339).unwrap_or_default(),
                metric_value: result.metric_value,
                title,
                body,
                delivery_status: status,
                delivery_error: error,
            };
            repo.record_alert_event(&event).map_err(|e| e.to_string())?;
        }
        let cutoff = now - Duration::days(i64::from(settings.notifications.retain_history_days));
        let _ = repo.prune_alert_history(&cutoff.format(&Rfc3339).unwrap_or_default());
        self.refresh_active_count()?;
        Ok(fired)
    }
}

fn context_for_rule(
    rule: &AlertRule,
    portfolio: &crate::portfolio::DerivedPortfolio,
    runtime: &crate::runtime::RuntimeSnapshot,
    now_ms: i64,
) -> AlertMetricContext {
    let runtime_holding = rule.symbol.as_ref().and_then(|symbol| {
        runtime.holdings.iter().find(|h| {
            h.symbol.eq_ignore_ascii_case(symbol)
                && rule
                    .exchange
                    .as_ref()
                    .map(|e| e.eq_ignore_ascii_case(&h.exchange))
                    .unwrap_or(true)
        })
    });
    let derived = rule.symbol.as_ref().and_then(|symbol| {
        portfolio.open_holdings().find(|h| {
            h.instrument.symbol.eq_ignore_ascii_case(symbol)
                && rule
                    .exchange
                    .as_ref()
                    .map(|e| e.eq_ignore_ascii_case(&h.instrument.exchange))
                    .unwrap_or(true)
        })
    });
    let raw_price = runtime_holding.and_then(|h| h.price);
    let quote_usable = runtime_holding
        .map(|h| quote_usable_for_trade_alerts(h.freshness))
        .unwrap_or(false);
    let price = if quote_usable { raw_price } else { None };
    let (holding_pnl, holding_return_pct) = holding_metrics(derived, price);
    let age = runtime_holding
        .and_then(|h| h.observed_unix_ms)
        .map(|t| now_ms.saturating_sub(t).max(0) as u64 / 1000);
    let portfolio_usable = portfolio.open_holdings().all(|holding| {
        runtime
            .holdings
            .iter()
            .find(|candidate| {
                candidate
                    .symbol
                    .eq_ignore_ascii_case(&holding.instrument.symbol)
                    && candidate
                        .exchange
                        .eq_ignore_ascii_case(&holding.instrument.exchange)
            })
            .map(|candidate| {
                quote_usable_for_trade_alerts(candidate.freshness) && candidate.price.is_some()
            })
            .unwrap_or(false)
    });
    AlertMetricContext {
        symbol: rule.symbol.clone(),
        exchange: rule.exchange.clone(),
        price,
        day_change_pct: if quote_usable {
            runtime_holding.and_then(|h| h.change_pct)
        } else {
            None
        },
        holding_return_pct,
        holding_pnl,
        quote_age_seconds: age,
        freshness: runtime_holding.map(|h| h.freshness),
        portfolio_value: if portfolio_usable {
            runtime.portfolio_value
        } else {
            None
        },
        portfolio_pnl: if portfolio_usable {
            runtime.total_pnl
        } else {
            None
        },
        market_session: runtime.market_session,
        now_unix_ms: now_ms,
    }
}
fn quote_usable_for_trade_alerts(freshness: crate::market::QuoteFreshness) -> bool {
    matches!(
        freshness,
        crate::market::QuoteFreshness::Live
            | crate::market::QuoteFreshness::Fresh
            | crate::market::QuoteFreshness::Delayed
    )
}
fn holding_metrics(holding: Option<&Holding>, price: Option<f64>) -> (Option<f64>, Option<f64>) {
    let (Some(h), Some(price)) = (holding, price) else {
        return (None, None);
    };
    let qty = h.quantity.to_f64().unwrap_or(0.0);
    let invested = h.gross_cost_basis().to_f64().unwrap_or(0.0);
    let pnl = price * qty - invested;
    let ret = if invested.abs() > f64::EPSILON {
        Some(pnl / invested * 100.0)
    } else {
        None
    };
    (Some(pnl), ret)
}
fn validate_rule(rule: &AlertRule) -> Result<(), String> {
    if !condition_threshold_valid(&rule.condition) {
        return Err("alert threshold is invalid for this condition".into());
    }
    if super::requires_symbol(&rule.condition) && (rule.symbol.is_none() || rule.exchange.is_none())
    {
        return Err("this alert condition requires both a symbol and exchange".into());
    }
    Ok(())
}
fn condition_threshold_valid(c: &super::AlertCondition) -> bool {
    match c {
        super::AlertCondition::PriceAbove { value }
        | super::AlertCondition::PriceBelow { value }
        | super::AlertCondition::PortfolioValueAbove { value }
        | super::AlertCondition::PortfolioValueBelow { value } => {
            value.is_finite() && *value >= 0.0
        }
        super::AlertCondition::HoldingPnlAbove { value }
        | super::AlertCondition::HoldingPnlBelow { value }
        | super::AlertCondition::PortfolioPnlAbove { value }
        | super::AlertCondition::PortfolioPnlBelow { value } => value.is_finite(),
        super::AlertCondition::DailyChangeAbove { percent }
        | super::AlertCondition::DailyChangeBelow { percent }
        | super::AlertCondition::HoldingReturnAbove { percent }
        | super::AlertCondition::HoldingReturnBelow { percent } => percent.is_finite(),
        super::AlertCondition::FeedStale { seconds } => *seconds > 0,
    }
}
fn requires_open_holding(c: &super::AlertCondition) -> bool {
    matches!(
        c,
        super::AlertCondition::HoldingReturnAbove { .. }
            | super::AlertCondition::HoldingReturnBelow { .. }
            | super::AlertCondition::HoldingPnlAbove { .. }
            | super::AlertCondition::HoldingPnlBelow { .. }
    )
}
fn now_unix_ms() -> i64 {
    OffsetDateTime::now_utc()
        .unix_timestamp_nanos()
        .saturating_div(1_000_000) as i64
}
