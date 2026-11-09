import type { DashboardSnapshot } from "./models";
import { emptySnapshot } from "./empty";

export async function loadDashboardSnapshot(): Promise<DashboardSnapshot> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<DashboardSnapshot>("dashboard_snapshot");
  } catch (error) {
    return { ...emptySnapshot, lastRefreshError: `Dashboard data unavailable: ${String(error)}` };
  }
}

export async function requestRefresh(): Promise<boolean> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<boolean>("request_refresh");
  } catch {
    return false;
  }
}

import type { AlertEventRecord, AlertRule, AppSettings, SettingsSaveResult } from "./models";

export async function loadUpstoxTokenConfigured(): Promise<boolean> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<boolean>("upstox_token_configured");
  } catch {
    return false;
  }
}

export async function loadSettings(): Promise<AppSettings | null> {
  try { const { invoke } = await import("@tauri-apps/api/core"); return await invoke<AppSettings>("get_settings"); } catch { return null; }
}
export async function persistSettings(settings: AppSettings): Promise<SettingsSaveResult> {
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<SettingsSaveResult>("save_settings", { settings });
}
export async function loadAlerts(): Promise<AlertRule[]> {
  try { const { invoke } = await import("@tauri-apps/api/core"); return await invoke<AlertRule[]>("list_alerts"); } catch { return []; }
}
export async function persistAlert(rule: AlertRule): Promise<AlertRule> {
  const { invoke } = await import("@tauri-apps/api/core"); return await invoke<AlertRule>("save_alert", { rule });
}
export async function removeAlert(id: string): Promise<boolean> {
  const { invoke } = await import("@tauri-apps/api/core"); return await invoke<boolean>("delete_alert", { id });
}
export async function loadAlertHistory(limit = 50): Promise<AlertEventRecord[]> {
  try { const { invoke } = await import("@tauri-apps/api/core"); return await invoke<AlertEventRecord[]>("alert_history", { limit }); } catch { return []; }
}
export async function sendTestNotification(): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core"); await invoke("test_notification");
}

import type { AnalyticsPreferences, AnalyticsSnapshot, CashFlowInput, CashFlowRecord } from "./models";

export async function loadAnalyticsSnapshot(): Promise<AnalyticsSnapshot | null> {
  try { const { invoke } = await import("@tauri-apps/api/core"); return await invoke<AnalyticsSnapshot>("analytics_snapshot"); } catch { return null; }
}
export async function loadCashFlows(): Promise<CashFlowRecord[]> {
  try { const { invoke } = await import("@tauri-apps/api/core"); return await invoke<CashFlowRecord[]>("list_cash_flows"); } catch { return []; }
}
export async function createCashFlow(input: CashFlowInput): Promise<CashFlowRecord> {
  const { invoke } = await import("@tauri-apps/api/core"); return await invoke<CashFlowRecord>("add_cash_flow", { input });
}
export async function removeCashFlow(id: number): Promise<boolean> {
  const { invoke } = await import("@tauri-apps/api/core"); return await invoke<boolean>("delete_cash_flow", { id });
}
export async function loadAnalyticsPreferences(): Promise<AnalyticsPreferences> {
  try { const { invoke } = await import("@tauri-apps/api/core"); return await invoke<AnalyticsPreferences>("get_analytics_preferences"); }
  catch { return { benchmarkExchange: null, benchmarkSymbol: null }; }
}
export async function persistAnalyticsPreferences(preferences: AnalyticsPreferences): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core"); await invoke("save_analytics_preferences", { preferences });
}

import type { PriceHistoryPoint } from "./models";
export async function loadHoldingPriceHistory(exchange: string, symbol: string, limit = 240): Promise<PriceHistoryPoint[]> {
  try { const { invoke } = await import("@tauri-apps/api/core"); return await invoke<PriceHistoryPoint[]>("holding_price_history", { exchange, symbol, limit }); } catch { return []; }
}


import type { BackupInfo, MaintenanceReport, ReliabilityStatus, StageRestoreResult } from "./models";
export async function loadReliabilityStatus(): Promise<ReliabilityStatus | null> {
  try { const { invoke } = await import("@tauri-apps/api/core"); return await invoke<ReliabilityStatus>("reliability_status"); } catch { return null; }
}
export async function runMaintenanceNow(): Promise<MaintenanceReport> {
  const { invoke } = await import("@tauri-apps/api/core"); return await invoke<MaintenanceReport>("run_maintenance_now");
}
export async function createBackupNow(): Promise<BackupInfo> {
  const { invoke } = await import("@tauri-apps/api/core"); return await invoke<BackupInfo>("create_backup_now");
}
export async function stageRestoreBackup(fileName: string): Promise<StageRestoreResult> {
  const { invoke } = await import("@tauri-apps/api/core"); return await invoke<StageRestoreResult>("stage_restore_backup", { fileName });
}
