mod service;
use serde::{Deserialize, Serialize};
pub use service::AlertService;
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

use crate::{
    market::QuoteFreshness, scheduler::MarketSessionState, settings::NotificationSettings,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AlertCondition {
    PriceAbove { value: f64 },
    PriceBelow { value: f64 },
    DailyChangeAbove { percent: f64 },
    DailyChangeBelow { percent: f64 },
    HoldingReturnAbove { percent: f64 },
    HoldingReturnBelow { percent: f64 },
    HoldingPnlAbove { value: f64 },
    HoldingPnlBelow { value: f64 },
    PortfolioValueAbove { value: f64 },
    PortfolioValueBelow { value: f64 },
    PortfolioPnlAbove { value: f64 },
    PortfolioPnlBelow { value: f64 },
    FeedStale { seconds: u64 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlertRepeatMode {
    Once,
    #[default]
    Crossing,
    RepeatWhileTrue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AlertRule {
    pub id: String,
    pub symbol: Option<String>,
    pub exchange: Option<String>,
    pub enabled: bool,
    pub condition: AlertCondition,
    pub cooldown_seconds: u64,
    pub reset_margin: Option<f64>,
    pub repeat_mode: AlertRepeatMode,
    pub market_hours_only: bool,
    pub armed: bool,
    pub last_value: Option<f64>,
    pub last_triggered_at: Option<String>,
    pub created_at: Option<String>,
}
impl Default for AlertRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            symbol: None,
            exchange: None,
            enabled: true,
            condition: AlertCondition::PriceAbove { value: 0.0 },
            cooldown_seconds: 900,
            reset_margin: None,
            repeat_mode: AlertRepeatMode::Crossing,
            market_hours_only: false,
            armed: true,
            last_value: None,
            last_triggered_at: None,
            created_at: None,
        }
    }
}
impl AlertRule {
    pub fn normalized(mut self) -> Self {
        self.id = self.id.trim().to_string();
        self.symbol = self
            .symbol
            .map(|v| v.trim().to_ascii_uppercase())
            .filter(|v| !v.is_empty());
        self.exchange = self
            .exchange
            .map(|v| v.trim().to_ascii_uppercase())
            .filter(|v| !v.is_empty());
        self.cooldown_seconds = self.cooldown_seconds.min(86_400);
        self.reset_margin = self.reset_margin.map(|v| v.abs()).filter(|v| v.is_finite());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertMetricContext {
    pub symbol: Option<String>,
    pub exchange: Option<String>,
    pub price: Option<f64>,
    pub day_change_pct: Option<f64>,
    pub holding_return_pct: Option<f64>,
    pub holding_pnl: Option<f64>,
    pub quote_age_seconds: Option<u64>,
    pub freshness: Option<QuoteFreshness>,
    pub portfolio_value: Option<f64>,
    pub portfolio_pnl: Option<f64>,
    pub market_session: MarketSessionState,
    pub now_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertEvaluation {
    pub metric_value: Option<f64>,
    pub active: bool,
    pub should_trigger: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertEvent {
    pub alert_id: String,
    pub fired_at: String,
    pub metric_value: Option<f64>,
    pub title: String,
    pub body: String,
    pub delivery_status: String,
    pub delivery_error: Option<String>,
}

pub fn evaluate(rule: &mut AlertRule, ctx: &AlertMetricContext) -> AlertEvaluation {
    if !rule.enabled {
        return evaluation(None, false, false, "disabled");
    }
    if rule.market_hours_only && ctx.market_session != MarketSessionState::Open {
        return evaluation(None, false, false, "outside_market_hours");
    }
    if requires_symbol(&rule.condition) && !same_instrument(rule, ctx) {
        return evaluation(None, false, false, "instrument_mismatch");
    }

    let metric = metric_value(&rule.condition, ctx);
    let Some(value) = metric else {
        return evaluation(None, false, false, "metric_unavailable");
    };
    let active = condition_active(&rule.condition, value);
    let previous = rule.last_value;
    let cooldown_ok = cooldown_elapsed(
        rule.last_triggered_at.as_deref(),
        ctx.now_unix_ms,
        rule.cooldown_seconds,
    );
    let crossed = previous
        .map(|v| !condition_active(&rule.condition, v) && active)
        .unwrap_or(false);
    let should_trigger = match rule.repeat_mode {
        AlertRepeatMode::Once => active && rule.last_triggered_at.is_none(),
        AlertRepeatMode::Crossing => active && rule.armed && crossed && cooldown_ok,
        AlertRepeatMode::RepeatWhileTrue => active && cooldown_ok,
    };

    if should_trigger {
        rule.last_triggered_at = Some(unix_ms_to_rfc3339(ctx.now_unix_ms));
        if rule.repeat_mode == AlertRepeatMode::Crossing {
            rule.armed = false;
        }
        if rule.repeat_mode == AlertRepeatMode::Once {
            rule.enabled = false;
        }
    } else if rule.repeat_mode == AlertRepeatMode::Crossing
        && !rule.armed
        && reset_reached(&rule.condition, value, rule.reset_margin.unwrap_or(0.0))
    {
        rule.armed = true;
    }
    rule.last_value = Some(value);
    evaluation(
        Some(value),
        active,
        should_trigger,
        if should_trigger {
            "triggered"
        } else if !cooldown_ok {
            "cooldown"
        } else if active {
            "active_not_triggered"
        } else {
            "inactive"
        },
    )
}

pub fn is_quiet_hours(settings: &NotificationSettings, now_utc: OffsetDateTime) -> bool {
    if !settings.enabled || !settings.quiet_hours_enabled {
        return false;
    }
    let ist = UtcOffset::from_hms(5, 30, 0).expect("valid IST");
    let local = now_utc.to_offset(ist);
    let current = u16::from(local.hour()) * 60 + u16::from(local.minute());
    let Some(start) = parse_hhmm(&settings.quiet_start) else {
        return false;
    };
    let Some(end) = parse_hhmm(&settings.quiet_end) else {
        return false;
    };
    if start == end {
        return true;
    }
    if start < end {
        current >= start && current < end
    } else {
        current >= start || current < end
    }
}

pub fn event_text(rule: &AlertRule, metric: f64) -> (String, String) {
    let target = rule.symbol.clone().unwrap_or_else(|| "Portfolio".into());
    let description = condition_description(&rule.condition);
    (
        format!("{target} alert"),
        format!("{description} · current {metric:.2}"),
    )
}

fn metric_value(condition: &AlertCondition, ctx: &AlertMetricContext) -> Option<f64> {
    match condition {
        AlertCondition::PriceAbove { .. } | AlertCondition::PriceBelow { .. } => ctx.price,
        AlertCondition::DailyChangeAbove { .. } | AlertCondition::DailyChangeBelow { .. } => {
            ctx.day_change_pct
        }
        AlertCondition::HoldingReturnAbove { .. } | AlertCondition::HoldingReturnBelow { .. } => {
            ctx.holding_return_pct
        }
        AlertCondition::HoldingPnlAbove { .. } | AlertCondition::HoldingPnlBelow { .. } => {
            ctx.holding_pnl
        }
        AlertCondition::PortfolioValueAbove { .. } | AlertCondition::PortfolioValueBelow { .. } => {
            ctx.portfolio_value
        }
        AlertCondition::PortfolioPnlAbove { .. } | AlertCondition::PortfolioPnlBelow { .. } => {
            ctx.portfolio_pnl
        }
        AlertCondition::FeedStale { .. } => ctx.quote_age_seconds.map(|v| v as f64),
    }
}
fn condition_active(condition: &AlertCondition, value: f64) -> bool {
    match condition {
        AlertCondition::PriceAbove { value: t }
        | AlertCondition::HoldingPnlAbove { value: t }
        | AlertCondition::PortfolioValueAbove { value: t }
        | AlertCondition::PortfolioPnlAbove { value: t } => value >= *t,
        AlertCondition::PriceBelow { value: t }
        | AlertCondition::HoldingPnlBelow { value: t }
        | AlertCondition::PortfolioValueBelow { value: t }
        | AlertCondition::PortfolioPnlBelow { value: t } => value <= *t,
        AlertCondition::DailyChangeAbove { percent: t }
        | AlertCondition::HoldingReturnAbove { percent: t } => value >= *t,
        AlertCondition::DailyChangeBelow { percent: t }
        | AlertCondition::HoldingReturnBelow { percent: t } => value <= *t,
        AlertCondition::FeedStale { seconds: t } => value >= *t as f64,
    }
}
fn threshold(condition: &AlertCondition) -> f64 {
    match condition {
        AlertCondition::PriceAbove { value }
        | AlertCondition::PriceBelow { value }
        | AlertCondition::HoldingPnlAbove { value }
        | AlertCondition::HoldingPnlBelow { value }
        | AlertCondition::PortfolioValueAbove { value }
        | AlertCondition::PortfolioValueBelow { value }
        | AlertCondition::PortfolioPnlAbove { value }
        | AlertCondition::PortfolioPnlBelow { value } => *value,
        AlertCondition::DailyChangeAbove { percent }
        | AlertCondition::DailyChangeBelow { percent }
        | AlertCondition::HoldingReturnAbove { percent }
        | AlertCondition::HoldingReturnBelow { percent } => *percent,
        AlertCondition::FeedStale { seconds } => *seconds as f64,
    }
}
fn is_above(condition: &AlertCondition) -> bool {
    matches!(
        condition,
        AlertCondition::PriceAbove { .. }
            | AlertCondition::DailyChangeAbove { .. }
            | AlertCondition::HoldingReturnAbove { .. }
            | AlertCondition::HoldingPnlAbove { .. }
            | AlertCondition::PortfolioValueAbove { .. }
            | AlertCondition::PortfolioPnlAbove { .. }
            | AlertCondition::FeedStale { .. }
    )
}
fn reset_reached(condition: &AlertCondition, value: f64, margin: f64) -> bool {
    let t = threshold(condition);
    if is_above(condition) {
        value <= t - margin
    } else {
        value >= t + margin
    }
}
pub(crate) fn requires_symbol(condition: &AlertCondition) -> bool {
    !matches!(
        condition,
        AlertCondition::PortfolioValueAbove { .. }
            | AlertCondition::PortfolioValueBelow { .. }
            | AlertCondition::PortfolioPnlAbove { .. }
            | AlertCondition::PortfolioPnlBelow { .. }
    )
}
fn same_instrument(rule: &AlertRule, ctx: &AlertMetricContext) -> bool {
    let symbol_ok = match (&rule.symbol, &ctx.symbol) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        (None, _) => true,
        _ => false,
    };
    let exchange_ok = match (&rule.exchange, &ctx.exchange) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        (None, _) => true,
        _ => false,
    };
    symbol_ok && exchange_ok
}
fn cooldown_elapsed(last: Option<&str>, now_ms: i64, cooldown: u64) -> bool {
    let Some(last) = last else { return true };
    let Ok(dt) = OffsetDateTime::parse(last, &Rfc3339) else {
        return true;
    };
    now_ms.saturating_sub(dt.unix_timestamp_nanos().saturating_div(1_000_000) as i64)
        >= cooldown.saturating_mul(1000) as i64
}
fn unix_ms_to_rfc3339(ms: i64) -> String {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .ok()
        .and_then(|v| v.format(&Rfc3339).ok())
        .unwrap_or_default()
}
fn evaluation(
    metric_value: Option<f64>,
    active: bool,
    should_trigger: bool,
    reason: &str,
) -> AlertEvaluation {
    AlertEvaluation {
        metric_value,
        active,
        should_trigger,
        reason: reason.into(),
    }
}
fn parse_hhmm(v: &str) -> Option<u16> {
    let (a, b) = v.split_once(':')?;
    let h = a.parse::<u16>().ok()?;
    let m = b.parse::<u16>().ok()?;
    if h < 24 && m < 60 {
        Some(h * 60 + m)
    } else {
        None
    }
}
fn condition_description(c: &AlertCondition) -> String {
    match c {
        AlertCondition::PriceAbove { value } => format!("Price above ₹{value:.2}"),
        AlertCondition::PriceBelow { value } => format!("Price below ₹{value:.2}"),
        AlertCondition::DailyChangeAbove { percent } => format!("Daily change above {percent:.2}%"),
        AlertCondition::DailyChangeBelow { percent } => format!("Daily change below {percent:.2}%"),
        AlertCondition::HoldingReturnAbove { percent } => format!("Return above {percent:.2}%"),
        AlertCondition::HoldingReturnBelow { percent } => format!("Return below {percent:.2}%"),
        AlertCondition::HoldingPnlAbove { value } => format!("Holding P&L above ₹{value:.2}"),
        AlertCondition::HoldingPnlBelow { value } => format!("Holding P&L below ₹{value:.2}"),
        AlertCondition::PortfolioValueAbove { value } => format!("Portfolio above ₹{value:.2}"),
        AlertCondition::PortfolioValueBelow { value } => format!("Portfolio below ₹{value:.2}"),
        AlertCondition::PortfolioPnlAbove { value } => format!("Portfolio P&L above ₹{value:.2}"),
        AlertCondition::PortfolioPnlBelow { value } => format!("Portfolio P&L below ₹{value:.2}"),
        AlertCondition::FeedStale { seconds } => format!("Feed stale for {seconds}s"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ctx(v: f64) -> AlertMetricContext {
        AlertMetricContext {
            symbol: Some("DEMOTECH".into()),
            exchange: Some("NSE".into()),
            price: Some(v),
            day_change_pct: Some(1.0),
            holding_return_pct: Some(10.0),
            holding_pnl: Some(20.0),
            quote_age_seconds: Some(5),
            freshness: Some(QuoteFreshness::Delayed),
            portfolio_value: Some(1000.0),
            portfolio_pnl: Some(100.0),
            market_session: MarketSessionState::Open,
            now_unix_ms: 1_800_000_000_000,
        }
    }
    #[test]
    fn crossing_does_not_fire_on_first_sample_above() {
        let mut r = AlertRule {
            symbol: Some("DEMOTECH".into()),
            condition: AlertCondition::PriceAbove { value: 300.0 },
            ..Default::default()
        };
        assert!(!evaluate(&mut r, &ctx(301.0)).should_trigger);
        let mut c = ctx(299.0);
        c.now_unix_ms += 1000;
        evaluate(&mut r, &c);
        c.price = Some(301.0);
        c.now_unix_ms += 1000;
        assert!(evaluate(&mut r, &c).should_trigger);
    }
    #[test]
    fn hysteresis_rearms_only_beyond_margin() {
        let mut r = AlertRule {
            symbol: Some("DEMOTECH".into()),
            condition: AlertCondition::PriceAbove { value: 300.0 },
            reset_margin: Some(2.0),
            last_value: Some(299.0),
            ..Default::default()
        };
        let mut c = ctx(301.0);
        assert!(evaluate(&mut r, &c).should_trigger);
        c.price = Some(299.0);
        c.now_unix_ms += 1_000_000;
        evaluate(&mut r, &c);
        assert!(!r.armed);
        c.price = Some(298.0);
        evaluate(&mut r, &c);
        assert!(r.armed);
    }
}
