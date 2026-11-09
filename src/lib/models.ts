export type QuoteFreshness = "live" | "fresh" | "delayed" | "stale" | "offline";
export type MarketSession = "open" | "closed" | "unknown";

export interface HoldingSummary {
  symbol: string;
  exchange: string;
  name?: string | null;
  quantity: number;
  averagePrice: number;
  invested: number;
  lastPrice: number | null;
  currentValue: number | null;
  dayChangePct: number | null;
  dayPnl: number | null;
  pnl: number | null;
  returnPct: number | null;
  allocationPct: number | null;
  freshness: QuoteFreshness;
  quoteSource: string | null;
  quoteObservedUnixMs: number | null;
}

export interface PortfolioSummary {
  invested: number;
  currentValue: number | null;
  totalPnl: number | null;
  totalReturnPct: number | null;
  dayPnl: number | null;
  dayReturnPct: number | null;
  realizedPnl: number;
  feesPaid: number;
  holdingsCount: number;
}

export interface DashboardSnapshot {
  source: "local_database" | "offline_demo";
  marketSession: MarketSession;
  providerId: string | null;
  lastRefreshUnixMs: number | null;
  refreshInFlight: boolean;
  lastRefreshError: string | null;
  activeAlertCount: number;
  portfolio: PortfolioSummary;
  holdings: HoldingSummary[];
}

export interface ChartPoint {
  label: string;
  value: number;
}

export type AlertRepeatMode = "once" | "crossing" | "repeat_while_true";
export type AlertCondition =
  | { kind: "price_above"; value: number }
  | { kind: "price_below"; value: number }
  | { kind: "daily_change_above"; percent: number }
  | { kind: "daily_change_below"; percent: number }
  | { kind: "holding_return_above"; percent: number }
  | { kind: "holding_return_below"; percent: number }
  | { kind: "holding_pnl_above"; value: number }
  | { kind: "holding_pnl_below"; value: number }
  | { kind: "portfolio_value_above"; value: number }
  | { kind: "portfolio_value_below"; value: number }
  | { kind: "portfolio_pnl_above"; value: number }
  | { kind: "portfolio_pnl_below"; value: number }
  | { kind: "feed_stale"; seconds: number };

export interface AlertRule {
  id: string;
  symbol: string | null;
  exchange: string | null;
  enabled: boolean;
  condition: AlertCondition;
  cooldown_seconds: number;
  reset_margin: number | null;
  repeat_mode: AlertRepeatMode;
  market_hours_only: boolean;
  armed: boolean;
  last_value: number | null;
  last_triggered_at: string | null;
  created_at: string | null;
}

export interface AlertEventRecord {
  id: number; alert_id: string; fired_at: string; metric_value: number | null;
  title: string; body: string; delivery_status: string; delivery_error: string | null;
}

export interface AppSettings {
  launch_at_startup: boolean;
  close_behavior: "destroy_to_tray" | "hide_to_tray";
  refresh: { open_seconds: number; closed_seconds: number | null; failure_backoff_max_seconds: number };
  market_data: { provider: string; request_timeout_seconds: number; stale_after_seconds: number; offline_after_seconds: number; history_enabled: boolean; refresh_on_startup: boolean };
  tray_symbols: string[];
  tray: { enabled: boolean; max_symbols: number; show_price: boolean; show_change_pct: boolean; show_portfolio_value: boolean; show_day_pnl: boolean; show_total_pnl: boolean; left_click_action: "open_dashboard" | "open_mini_mode" | "none" };
  notifications: { enabled: boolean; sound: boolean; quiet_hours_enabled: boolean; quiet_start: string; quiet_end: string; default_cooldown_seconds: number; retain_history_days: number };
  reliability: { backup_enabled: boolean; backup_interval_hours: number; backup_keep_count: number; quote_history_retention_days: number; performance_retention_days: number; maintenance_interval_minutes: number; event_log_max_bytes: number; event_log_keep_files: number };
}

export interface SettingsSaveResult { restartRequired: boolean; }

export type CashFlowKind = "deposit" | "withdrawal" | "dividend" | "interest";
export interface CashFlowRecord { id: number; kind: CashFlowKind; amount: number; occurredAt: string; note: string | null; }
export interface CashFlowInput { kind: CashFlowKind; amount: string; occurredAt: string; note?: string | null; }
export interface PerformancePoint { observedAt: string; portfolioValue: number; cumulativeExternalFlow: number; gainSinceStart: number; }
export interface HoldingContribution { symbol: string; exchange: string; currentValue: number; unrealizedPnl: number; allocationPct: number; pnlContributionPct: number | null; }
export interface TransactionAnalytics { buyCount: number; sellCount: number; grossBuys: number; grossSells: number; recordedFees: number; }
export interface BenchmarkSummary { exchange: string; symbol: string; startPrice: number; endPrice: number; returnPct: number; portfolioExcessReturnPct: number | null; }
export interface AnalyticsSnapshot {
  currentValue: number | null;
  openCostBasis: number;
  unrealizedPnl: number | null;
  realizedGrossPnl: number;
  recordedFees: number;
  investmentIncome: number;
  netTotalPnl: number | null;
  cashFlowAdjustedReturnPct: number | null;
  netExternalContributions: number;
  deposits: number;
  withdrawals: number;
  snapshotCount: number;
  firstSnapshotAt: string | null;
  lastSnapshotAt: string | null;
  performance: PerformancePoint[];
  contributions: HoldingContribution[];
  transactions: TransactionAnalytics;
  benchmark: BenchmarkSummary | null;
  methodology: string;
}
export interface AnalyticsPreferences { benchmarkExchange: string | null; benchmarkSymbol: string | null; }

export interface PriceHistoryPoint { observedAt: string; price: number; }


export interface RuntimeDiagnostics {
  refresh_success_count: number; refresh_error_count: number; last_refresh_duration_ms: number | null; wake_recovery_count: number;
}
export interface DatabaseHealth { ok: boolean; quickCheck: string; schemaVersion: number; databaseBytes: number; }
export interface BackupInfo { path: string; fileName: string; bytes: number; modifiedUnixMs: number; verified: boolean; }
export interface ReliabilityStatus { database: DatabaseHealth; backups: BackupInfo[]; pendingRestore: boolean; lastMaintenanceAt: string | null; runtime: RuntimeDiagnostics & Record<string, unknown>; }
export interface MaintenanceReport { integrityOk: boolean; backupCreated: string | null; backupsRemoved: number; quoteHistoryRemoved: number; performanceSnapshotsRemoved: number; alertEventsRemoved: number; walCheckpointed: boolean; }
export interface StageRestoreResult { restartRequired: boolean; schemaVersion: number; }
