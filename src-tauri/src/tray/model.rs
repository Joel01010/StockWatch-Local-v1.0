use serde::{Deserialize, Serialize};

use crate::{
    market::QuoteFreshness,
    runtime::{RuntimeHolding, RuntimeSnapshot},
    scheduler::MarketSessionState,
    settings::AppSettings,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrayAction {
    OpenDashboard,
    OpenMiniMode,
    OpenSettings,
    RefreshNow,
    Exit,
    OpenSymbol { symbol: String },
    OpenAlerts,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrayMenuKind {
    Display,
    Action,
    Separator,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrayMenuEntry {
    pub id: String,
    pub label: String,
    pub kind: TrayMenuKind,
    pub enabled: bool,
}

impl TrayMenuEntry {
    fn display(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: TrayMenuKind::Display,
            enabled: false,
        }
    }

    fn action(id: impl Into<String>, label: impl Into<String>, enabled: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: TrayMenuKind::Action,
            enabled,
        }
    }

    fn separator(index: usize) -> Self {
        Self {
            id: format!("separator:{index}"),
            label: String::new(),
            kind: TrayMenuKind::Separator,
            enabled: false,
        }
    }
}

pub fn build_tray_menu_model(
    snapshot: &RuntimeSnapshot,
    settings: &AppSettings,
) -> Vec<TrayMenuEntry> {
    let mut entries = Vec::new();
    entries.push(TrayMenuEntry::display("status:product", "StockWatch Local"));
    entries.push(TrayMenuEntry::display(
        "status:market",
        format!(
            "Market: {} · {}",
            market_label(snapshot.market_session),
            updated_label(snapshot)
        ),
    ));
    entries.push(TrayMenuEntry::separator(entries.len()));

    let symbols = settings.effective_tray_symbols();
    let holdings: Vec<&RuntimeHolding> = if symbols.is_empty() {
        snapshot
            .holdings
            .iter()
            .take(settings.tray.max_symbols.clamp(1, 20))
            .collect()
    } else {
        symbols
            .iter()
            .filter_map(|symbol| {
                snapshot
                    .holdings
                    .iter()
                    .find(|h| h.symbol.eq_ignore_ascii_case(symbol))
            })
            .collect()
    };

    if holdings.is_empty() {
        entries.push(TrayMenuEntry::display(
            "status:no-holdings",
            "No tray holdings configured",
        ));
    } else {
        for holding in holdings {
            entries.push(TrayMenuEntry::action(
                format!("symbol:{}", holding.symbol.to_ascii_uppercase()),
                holding_label(holding, settings),
                true,
            ));
        }
    }

    entries.push(TrayMenuEntry::separator(entries.len()));
    if settings.tray.show_portfolio_value {
        entries.push(TrayMenuEntry::display(
            "status:portfolio",
            format!("Portfolio   {}", money(snapshot.portfolio_value)),
        ));
    }
    if settings.tray.show_day_pnl {
        entries.push(TrayMenuEntry::display(
            "status:day-pnl",
            format!("Today       {}", signed_money(snapshot.day_pnl)),
        ));
    }
    if settings.tray.show_total_pnl {
        entries.push(TrayMenuEntry::display(
            "status:total-pnl",
            format!("Total       {}", signed_money(snapshot.total_pnl)),
        ));
    }

    entries.push(TrayMenuEntry::separator(entries.len()));
    entries.push(TrayMenuEntry::action(
        "action:alerts",
        format!(
            "Alerts{}",
            if snapshot.active_alert_count > 0 {
                format!(" ({})", snapshot.active_alert_count)
            } else {
                String::new()
            }
        ),
        true,
    ));
    entries.push(TrayMenuEntry::action(
        "action:refresh",
        if snapshot.refresh_in_flight {
            "Refreshing…"
        } else {
            "Refresh Now"
        },
        !snapshot.refresh_in_flight,
    ));
    entries.push(TrayMenuEntry::action(
        "action:dashboard",
        "Show Dashboard",
        true,
    ));
    entries.push(TrayMenuEntry::action("action:mini", "Quick View", true));
    entries.push(TrayMenuEntry::action("action:settings", "Settings", true));
    entries.push(TrayMenuEntry::separator(entries.len()));
    entries.push(TrayMenuEntry::action("action:exit", "Exit", true));
    entries
}

pub fn parse_tray_action(id: &str) -> Option<TrayAction> {
    match id {
        "action:dashboard" => Some(TrayAction::OpenDashboard),
        "action:mini" => Some(TrayAction::OpenMiniMode),
        "action:settings" => Some(TrayAction::OpenSettings),
        "action:refresh" => Some(TrayAction::RefreshNow),
        "action:alerts" => Some(TrayAction::OpenAlerts),
        "action:exit" => Some(TrayAction::Exit),
        _ => id
            .strip_prefix("symbol:")
            .filter(|value| !value.is_empty())
            .map(|symbol| TrayAction::OpenSymbol {
                symbol: symbol.to_string(),
            }),
    }
}

fn holding_label(holding: &RuntimeHolding, settings: &AppSettings) -> String {
    let mut parts = vec![holding.symbol.to_ascii_uppercase()];
    if settings.tray.show_price {
        parts.push(match holding.price {
            Some(price) => format!("₹{price:.2}"),
            None => "—".into(),
        });
    }
    if settings.tray.show_change_pct {
        parts.push(match holding.change_pct {
            Some(change) => format!("{}{change:.2}%", if change >= 0.0 { "+" } else { "" }),
            None => "—".into(),
        });
    }
    if matches!(
        holding.freshness,
        QuoteFreshness::Stale | QuoteFreshness::Offline
    ) {
        parts.push(
            match holding.freshness {
                QuoteFreshness::Offline => "OFFLINE",
                _ => "STALE",
            }
            .into(),
        );
    }
    parts.join("   ")
}

fn market_label(state: MarketSessionState) -> &'static str {
    match state {
        MarketSessionState::Open => "OPEN",
        MarketSessionState::Closed => "CLOSED",
        MarketSessionState::Unknown => "UNKNOWN",
    }
}

fn updated_label(snapshot: &RuntimeSnapshot) -> String {
    if snapshot.refresh_in_flight {
        return "refreshing".into();
    }
    if snapshot.last_refresh_error.is_some() {
        return "feed error".into();
    }
    match snapshot.last_refresh_unix_ms {
        Some(_) => "updated".into(),
        None => "not updated".into(),
    }
}

fn money(value: Option<f64>) -> String {
    value
        .map(|v| format!("₹{v:.2}"))
        .unwrap_or_else(|| "—".into())
}

fn signed_money(value: Option<f64>) -> String {
    value
        .map(|v| format!("{}₹{:.2}", if v >= 0.0 { "+" } else { "−" }, v.abs()))
        .unwrap_or_else(|| "—".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> RuntimeSnapshot {
        RuntimeSnapshot {
            market_session: MarketSessionState::Open,
            holdings: vec![RuntimeHolding {
                symbol: "DEMOTECH".into(),
                exchange: "NSE".into(),
                price: Some(210.00),
                change_pct: Some(1.20),
                freshness: QuoteFreshness::Fresh,
                source: Some("test".into()),
                observed_unix_ms: Some(1_799_999_000_000),
            }],
            portfolio_value: Some(2557.00),
            day_pnl: Some(7.38),
            total_pnl: Some(56.99),
            ..RuntimeSnapshot::default()
        }
    }

    #[test]
    fn menu_contains_expected_primary_actions() {
        let model = build_tray_menu_model(&snapshot(), &AppSettings::default());
        let ids: Vec<&str> = model.iter().map(|entry| entry.id.as_str()).collect();
        assert!(ids.contains(&"action:refresh"));
        assert!(ids.contains(&"action:dashboard"));
        assert!(ids.contains(&"action:settings"));
        assert!(ids.contains(&"action:mini"));
        assert!(ids.contains(&"action:exit"));
    }

    #[test]
    fn refresh_is_disabled_while_refreshing() {
        let mut state = snapshot();
        state.refresh_in_flight = true;
        let model = build_tray_menu_model(&state, &AppSettings::default());
        let refresh = model
            .iter()
            .find(|entry| entry.id == "action:refresh")
            .unwrap();
        assert!(!refresh.enabled);
        assert_eq!(refresh.label, "Refreshing…");
    }

    #[test]
    fn parser_ignores_display_only_ids() {
        assert_eq!(parse_tray_action("status:market"), None);
        assert_eq!(parse_tray_action("action:exit"), Some(TrayAction::Exit));
        assert_eq!(
            parse_tray_action("symbol:DEMOTECH"),
            Some(TrayAction::OpenSymbol {
                symbol: "DEMOTECH".into()
            })
        );
    }
}
