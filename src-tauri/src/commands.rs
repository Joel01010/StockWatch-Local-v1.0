use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::Deserialize;
use serde::Serialize;
use std::str::FromStr;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

use crate::{
    alerts::{AlertRule, AlertService},
    analytics::{
        build_analytics_snapshot, AnalyticsSnapshot, BenchmarkSummary, CashFlow, CashFlowKind,
    },
    imports::{opening_holdings_to_trades, parse_holdings_csv},
    market::{MarketDataService, QuoteFreshness},
    portfolio::{derive_portfolio, DerivedPortfolio, Trade},
    reliability::{self, BackupInfo, DatabaseHealth, MaintenanceReport, MaintenanceService},
    runtime::{RuntimeSnapshot, RuntimeState},
    scheduler::MarketSessionState,
    settings::{AppSettings, SettingsState},
    storage::{PortfolioRepository, SCHEMA_VERSION},
    tray::{build_tray_menu_model, refresh_native_tray},
};

/// Pure calculation command used by the Svelte layer. It does not touch the network or disk.
#[tauri::command]
pub fn calculate_portfolio(trades: Vec<Trade>) -> Result<DerivedPortfolio, String> {
    derive_portfolio(&trades).map_err(|error| error.to_string())
}

/// Parses an opening-holdings CSV payload and returns the synthetic opening trades that would
/// be persisted. Persistence is intentionally separate so a future UI can preview before commit.
#[tauri::command]
pub fn preview_opening_holdings(
    csv_text: String,
    observed_at: String,
    import_batch: String,
) -> Result<Vec<Trade>, String> {
    let holdings = parse_holdings_csv(csv_text.as_bytes()).map_err(|error| error.to_string())?;
    opening_holdings_to_trades(&holdings, &observed_at, &import_batch)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn schema_version() -> i64 {
    SCHEMA_VERSION
}

#[tauri::command]
pub fn runtime_snapshot(state: tauri::State<'_, RuntimeState>) -> RuntimeSnapshot {
    state.snapshot()
}

/// Exposes the platform-neutral tray model for the Settings preview and automated tests.
#[tauri::command]
pub fn preview_tray_menu(
    snapshot: RuntimeSnapshot,
    settings: AppSettings,
) -> Vec<crate::tray::TrayMenuEntry> {
    build_tray_menu_model(&snapshot, &settings)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardHolding {
    pub symbol: String,
    pub exchange: String,
    pub name: Option<String>,
    pub quantity: f64,
    pub average_price: f64,
    pub invested: f64,
    pub last_price: Option<f64>,
    pub current_value: Option<f64>,
    pub day_change_pct: Option<f64>,
    pub day_pnl: Option<f64>,
    pub pnl: Option<f64>,
    pub return_pct: Option<f64>,
    pub allocation_pct: Option<f64>,
    pub freshness: QuoteFreshness,
    pub quote_source: Option<String>,
    pub quote_observed_unix_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardPortfolio {
    pub invested: f64,
    pub current_value: Option<f64>,
    pub total_pnl: Option<f64>,
    pub total_return_pct: Option<f64>,
    pub day_pnl: Option<f64>,
    pub day_return_pct: Option<f64>,
    pub realized_pnl: f64,
    pub fees_paid: f64,
    pub holdings_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub source: &'static str,
    pub market_session: &'static str,
    pub last_refresh_unix_ms: Option<i64>,
    pub refresh_in_flight: bool,
    pub last_refresh_error: Option<String>,
    pub active_alert_count: usize,
    pub provider_id: Option<String>,
    pub portfolio: DashboardPortfolio,
    pub holdings: Vec<DashboardHolding>,
}

/// Reads the canonical local transaction ledger and merges it with the in-memory quote/runtime
/// snapshot. This keeps the Phase 3 UI dependent on local deterministic state rather than
/// embedding portfolio math in Svelte.
#[tauri::command]
pub fn dashboard_snapshot(
    app: tauri::AppHandle,
    state: tauri::State<'_, RuntimeState>,
) -> Result<DashboardSnapshot, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&app_data).map_err(|error| error.to_string())?;
    let repo = PortfolioRepository::open(app_data.join("stockwatch.db"))
        .map_err(|error| error.to_string())?;
    let derived = repo.derive_portfolio().map_err(|error| error.to_string())?;
    let runtime = state.snapshot();

    let mut rows: Vec<DashboardHolding> = derived
        .open_holdings()
        .map(|holding| {
            let quantity = decimal_f64(holding.quantity);
            let average_price = decimal_f64(holding.average_price);
            let invested = decimal_f64(holding.gross_cost_basis());
            let runtime_holding = runtime.holdings.iter().find(|candidate| {
                candidate
                    .symbol
                    .eq_ignore_ascii_case(&holding.instrument.symbol)
                    && candidate
                        .exchange
                        .eq_ignore_ascii_case(&holding.instrument.exchange)
            });
            let last_price = runtime_holding.and_then(|row| row.price);
            let day_change_pct = runtime_holding.and_then(|row| row.change_pct);
            let current_value = last_price.map(|price| price * quantity);
            let pnl = current_value.map(|value| value - invested);
            let return_pct = pnl.and_then(|value| {
                if invested.abs() > f64::EPSILON {
                    Some(value / invested * 100.0)
                } else {
                    None
                }
            });
            let day_pnl = match (last_price, day_change_pct) {
                (Some(price), Some(change)) if (1.0 + change / 100.0).abs() > f64::EPSILON => {
                    let prior = price / (1.0 + change / 100.0);
                    Some((price - prior) * quantity)
                }
                _ => None,
            };
            DashboardHolding {
                symbol: holding.instrument.symbol.clone(),
                exchange: holding.instrument.exchange.clone(),
                name: holding.instrument.name.clone(),
                quantity,
                average_price,
                invested,
                last_price,
                current_value,
                day_change_pct,
                day_pnl,
                pnl,
                return_pct,
                allocation_pct: None,
                freshness: runtime_holding
                    .map(|row| row.freshness)
                    .unwrap_or(QuoteFreshness::Offline),
                quote_source: runtime_holding.and_then(|row| row.source.clone()),
                quote_observed_unix_ms: runtime_holding.and_then(|row| row.observed_unix_ms),
            }
        })
        .collect();

    let invested: f64 = rows.iter().map(|row| row.invested).sum();
    let priced_count = rows
        .iter()
        .filter(|row| row.current_value.is_some())
        .count();
    let current_value = if rows.is_empty() || priced_count != rows.len() {
        None
    } else {
        Some(rows.iter().filter_map(|row| row.current_value).sum())
    };
    if let Some(total) = current_value.filter(|value: &f64| value.abs() > f64::EPSILON) {
        for row in &mut rows {
            row.allocation_pct = row.current_value.map(|value| value / total * 100.0);
        }
    }
    let total_pnl = current_value.map(|value| value - invested);
    let total_return_pct = total_pnl.and_then(|value| {
        if invested.abs() > f64::EPSILON {
            Some(value / invested * 100.0)
        } else {
            None
        }
    });
    let day_values: Vec<f64> = rows.iter().filter_map(|row| row.day_pnl).collect();
    let day_pnl = if day_values.is_empty() {
        None
    } else {
        Some(day_values.iter().sum())
    };
    let prior_value: Option<f64> = match (current_value, day_pnl) {
        (Some(current), Some(day)) => Some(current - day),
        _ => None,
    };
    let day_return_pct = match (day_pnl, prior_value) {
        (Some(day), Some(prior)) if prior.abs() > f64::EPSILON => Some(day / prior * 100.0),
        _ => None,
    };

    Ok(DashboardSnapshot {
        source: "local_database",
        market_session: market_session_label(runtime.market_session),
        last_refresh_unix_ms: runtime.last_refresh_unix_ms,
        refresh_in_flight: runtime.refresh_in_flight,
        last_refresh_error: runtime.last_refresh_error,
        active_alert_count: runtime.active_alert_count,
        provider_id: runtime.provider_id,
        portfolio: DashboardPortfolio {
            invested,
            current_value,
            total_pnl,
            total_return_pct,
            day_pnl,
            day_return_pct,
            realized_pnl: decimal_f64(derived.realized_gross_pnl),
            fees_paid: decimal_f64(derived.fees_paid),
            holdings_count: rows.len(),
        },
        holdings: rows,
    })
}

/// Triggers the configured Phase 4 market-data provider. Duplicate refreshes are rejected by
/// the shared runtime gate rather than spawning overlapping network requests.
#[tauri::command]
pub async fn request_refresh(
    app: tauri::AppHandle,
    service: tauri::State<'_, MarketDataService>,
    alerts: tauri::State<'_, AlertService>,
) -> Result<bool, String> {
    match service.refresh_once().await {
        Ok(_) => {
            let _ = alerts.evaluate_all(&app);
            Ok(true)
        }
        Err(crate::market::MarketDataError::RefreshAlreadyInFlight) => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}

/// Stores an explicit provider symbol mapping for an already-known instrument. This is useful
/// when a public feed does not use StockWatch's default NSE/BSE suffix convention.
#[tauri::command]
pub fn set_market_symbol_override(
    app: tauri::AppHandle,
    exchange: String,
    symbol: String,
    provider_id: String,
    provider_symbol: String,
) -> Result<(), String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&app_data).map_err(|error| error.to_string())?;
    let repo = PortfolioRepository::open(app_data.join("stockwatch.db"))
        .map_err(|error| error.to_string())?;
    repo.set_provider_symbol(&exchange, &symbol, &provider_id, &provider_symbol)
        .map_err(|error| error.to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSaveResult {
    pub restart_required: bool,
}

#[tauri::command]
pub fn get_settings(settings: tauri::State<'_, SettingsState>) -> AppSettings {
    settings.snapshot()
}

#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    runtime: tauri::State<'_, RuntimeState>,
    state: tauri::State<'_, SettingsState>,
    settings: AppSettings,
) -> Result<SettingsSaveResult, String> {
    let previous = state.snapshot();
    let normalized = settings.normalized();
    if previous.launch_at_startup != normalized.launch_at_startup {
        let manager = app.autolaunch();
        let result = if normalized.launch_at_startup {
            manager.enable()
        } else {
            manager.disable()
        };
        result.map_err(|e| format!("failed to update native startup registration: {e}"))?;
    }
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let repo =
        PortfolioRepository::open(app_data.join("stockwatch.db")).map_err(|e| e.to_string())?;
    repo.save_app_settings(&normalized)
        .map_err(|e| e.to_string())?;
    let restart_required = previous.market_data != normalized.market_data
        || previous.reliability.event_log_max_bytes != normalized.reliability.event_log_max_bytes
        || previous.reliability.event_log_keep_files != normalized.reliability.event_log_keep_files;
    state.replace(normalized);
    let _ = refresh_native_tray(&app, runtime.inner(), state.inner());
    Ok(SettingsSaveResult { restart_required })
}

#[tauri::command]
pub fn list_alerts(alerts: tauri::State<'_, AlertService>) -> Result<Vec<AlertRule>, String> {
    alerts.list_rules()
}

#[tauri::command]
pub fn save_alert(
    app: tauri::AppHandle,
    runtime: tauri::State<'_, RuntimeState>,
    settings: tauri::State<'_, SettingsState>,
    alerts: tauri::State<'_, AlertService>,
    rule: AlertRule,
) -> Result<AlertRule, String> {
    let saved = alerts.save_rule(rule)?;
    let _ = refresh_native_tray(&app, runtime.inner(), settings.inner());
    Ok(saved)
}

#[tauri::command]
pub fn delete_alert(
    app: tauri::AppHandle,
    runtime: tauri::State<'_, RuntimeState>,
    settings: tauri::State<'_, SettingsState>,
    alerts: tauri::State<'_, AlertService>,
    id: String,
) -> Result<bool, String> {
    let deleted = alerts.delete_rule(&id)?;
    let _ = refresh_native_tray(&app, runtime.inner(), settings.inner());
    Ok(deleted)
}

#[tauri::command]
pub fn alert_history(
    alerts: tauri::State<'_, AlertService>,
    limit: Option<usize>,
) -> Result<Vec<crate::storage::AlertEventRecord>, String> {
    alerts.recent_events(limit.unwrap_or(50))
}

#[tauri::command]
pub fn test_notification(
    app: tauri::AppHandle,
    settings: tauri::State<'_, SettingsState>,
) -> Result<(), String> {
    let current = settings.snapshot();
    if !current.notifications.enabled {
        return Err("notifications are disabled in settings".into());
    }
    crate::notifications::send_native(
        &app,
        "StockWatch Local",
        "Test notification from your local alert settings",
        current.notifications.sound,
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashFlowInput {
    pub kind: CashFlowKind,
    pub amount: String,
    pub occurred_at: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashFlowRecord {
    pub id: i64,
    pub kind: CashFlowKind,
    pub amount: f64,
    pub occurred_at: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsPreferences {
    pub benchmark_exchange: Option<String>,
    pub benchmark_symbol: Option<String>,
}

#[tauri::command]
pub fn analytics_snapshot(
    app: tauri::AppHandle,
    runtime: tauri::State<'_, RuntimeState>,
) -> Result<AnalyticsSnapshot, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let repo =
        PortfolioRepository::open(app_data.join("stockwatch.db")).map_err(|e| e.to_string())?;
    let portfolio = repo.derive_portfolio().map_err(|e| e.to_string())?;
    let trades = repo.list_trades().map_err(|e| e.to_string())?;
    let cash_flows = repo.list_cash_flows().map_err(|e| e.to_string())?;
    let snapshots = repo
        .performance_snapshots(20_000)
        .map_err(|e| e.to_string())?;
    let runtime = runtime.snapshot();
    let prices: Vec<(String, String, f64)> = runtime
        .holdings
        .iter()
        .filter_map(|row| {
            if !matches!(
                row.freshness,
                QuoteFreshness::Live | QuoteFreshness::Fresh | QuoteFreshness::Delayed
            ) {
                return None;
            }
            row.price
                .map(|price| (row.exchange.clone(), row.symbol.clone(), price))
        })
        .collect();
    let benchmark = match (
        repo.analytics_benchmark().map_err(|e| e.to_string())?,
        snapshots.first(),
        snapshots.last(),
    ) {
        (Some((exchange, symbol)), Some(first), Some(last)) => repo
            .benchmark_price_range(&exchange, &symbol, &first.observed_at, &last.observed_at)
            .map_err(|e| e.to_string())?
            .map(|(start_price, end_price)| BenchmarkSummary {
                exchange,
                symbol,
                start_price,
                end_price,
                return_pct: (end_price / start_price - 1.0) * 100.0,
                portfolio_excess_return_pct: None,
            }),
        _ => None,
    };
    Ok(build_analytics_snapshot(
        &portfolio,
        &prices,
        &cash_flows,
        &snapshots,
        benchmark,
        &trades,
    ))
}

#[tauri::command]
pub fn list_cash_flows(app: tauri::AppHandle) -> Result<Vec<CashFlowRecord>, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let repo =
        PortfolioRepository::open(app_data.join("stockwatch.db")).map_err(|e| e.to_string())?;
    repo.list_cash_flows()
        .map_err(|e| e.to_string())
        .map(|rows| {
            rows.into_iter()
                .filter_map(|flow| {
                    Some(CashFlowRecord {
                        id: flow.id?,
                        kind: flow.kind,
                        amount: flow.amount.to_f64().unwrap_or(0.0),
                        occurred_at: flow.occurred_at,
                        note: flow.note,
                    })
                })
                .collect()
        })
}

#[tauri::command]
pub fn add_cash_flow(
    app: tauri::AppHandle,
    input: CashFlowInput,
) -> Result<CashFlowRecord, String> {
    let amount = Decimal::from_str(input.amount.trim())
        .map_err(|_| "amount must be a valid positive decimal".to_string())?;
    if amount <= Decimal::ZERO {
        return Err("amount must be greater than zero".into());
    }
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let repo =
        PortfolioRepository::open(app_data.join("stockwatch.db")).map_err(|e| e.to_string())?;
    let flow = CashFlow {
        id: None,
        kind: input.kind,
        amount,
        occurred_at: input.occurred_at,
        note: input.note,
        external_ref: None,
    };
    let id = repo.insert_cash_flow(&flow).map_err(|e| e.to_string())?;
    Ok(CashFlowRecord {
        id,
        kind: flow.kind,
        amount: flow.amount.to_f64().unwrap_or(0.0),
        occurred_at: flow.occurred_at,
        note: flow.note,
    })
}

#[tauri::command]
pub fn delete_cash_flow(app: tauri::AppHandle, id: i64) -> Result<bool, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let repo =
        PortfolioRepository::open(app_data.join("stockwatch.db")).map_err(|e| e.to_string())?;
    repo.delete_cash_flow(id).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceHistoryPoint {
    pub observed_at: String,
    pub price: f64,
}

#[tauri::command]
pub fn holding_price_history(
    app: tauri::AppHandle,
    exchange: String,
    symbol: String,
    limit: Option<usize>,
) -> Result<Vec<PriceHistoryPoint>, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let repo =
        PortfolioRepository::open(app_data.join("stockwatch.db")).map_err(|e| e.to_string())?;
    repo.quote_history_points(&exchange, &symbol, limit.unwrap_or(240))
        .map_err(|e| e.to_string())
        .map(|rows| {
            rows.into_iter()
                .map(|(observed_at, price)| PriceHistoryPoint { observed_at, price })
                .collect()
        })
}

#[tauri::command]
pub fn get_analytics_preferences(app: tauri::AppHandle) -> Result<AnalyticsPreferences, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let repo =
        PortfolioRepository::open(app_data.join("stockwatch.db")).map_err(|e| e.to_string())?;
    let benchmark = repo.analytics_benchmark().map_err(|e| e.to_string())?;
    Ok(match benchmark {
        Some((exchange, symbol)) => AnalyticsPreferences {
            benchmark_exchange: Some(exchange),
            benchmark_symbol: Some(symbol),
        },
        None => AnalyticsPreferences {
            benchmark_exchange: None,
            benchmark_symbol: None,
        },
    })
}

#[tauri::command]
pub fn save_analytics_preferences(
    app: tauri::AppHandle,
    preferences: AnalyticsPreferences,
) -> Result<(), String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let repo =
        PortfolioRepository::open(app_data.join("stockwatch.db")).map_err(|e| e.to_string())?;
    repo.set_analytics_benchmark(
        preferences.benchmark_exchange.as_deref(),
        preferences.benchmark_symbol.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReliabilityStatus {
    pub database: DatabaseHealth,
    pub backups: Vec<BackupInfo>,
    pub pending_restore: bool,
    pub last_maintenance_at: Option<String>,
    pub runtime: RuntimeSnapshot,
}

#[tauri::command]
pub fn reliability_status(
    app: tauri::AppHandle,
    runtime: tauri::State<'_, RuntimeState>,
) -> Result<ReliabilityStatus, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_data.join("stockwatch.db");
    let database = reliability::inspect_database(&db_path)?;
    let backups = reliability::list_backups(&app_data.join("backups"))?;
    let pending_restore = app_data.join("stockwatch.restore.pending.db").exists();
    let last_maintenance_at = PortfolioRepository::open(&db_path).ok().and_then(|repo| {
        repo.reliability_state("last_maintenance")
            .ok()
            .flatten()
            .map(|(_, at)| at)
    });
    Ok(ReliabilityStatus {
        database,
        backups,
        pending_restore,
        last_maintenance_at,
        runtime: runtime.snapshot(),
    })
}

#[tauri::command]
pub fn run_maintenance_now(
    service: tauri::State<'_, MaintenanceService>,
) -> Result<MaintenanceReport, String> {
    service.run_once()
}

#[tauri::command]
pub fn create_backup_now(app: tauri::AppHandle) -> Result<BackupInfo, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    reliability::create_backup(
        &app_data.join("stockwatch.db"),
        &app_data.join("backups"),
        "manual",
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageRestoreResult {
    pub restart_required: bool,
    pub schema_version: i64,
}

#[tauri::command]
pub fn stage_restore_backup(
    app: tauri::AppHandle,
    file_name: String,
) -> Result<StageRestoreResult, String> {
    if file_name.is_empty()
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains("..")
    {
        return Err("invalid backup file name".into());
    }
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup = app_data.join("backups").join(&file_name);
    let health = reliability::stage_restore(&app_data, &backup)?;
    Ok(StageRestoreResult {
        restart_required: true,
        schema_version: health.schema_version,
    })
}

fn decimal_f64(value: rust_decimal::Decimal) -> f64 {
    value.to_f64().unwrap_or(0.0)
}

fn market_session_label(state: MarketSessionState) -> &'static str {
    match state {
        MarketSessionState::Open => "open",
        MarketSessionState::Closed => "closed",
        MarketSessionState::Unknown => "unknown",
    }
}
