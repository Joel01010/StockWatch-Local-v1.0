use crate::scheduler::RefreshPolicy;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

fn default_true() -> bool {
    true
}
fn default_max_tray_symbols() -> usize {
    8
}
fn default_provider() -> String {
    "auto".into()
}
fn default_timeout() -> u64 {
    8
}
fn default_stale_after() -> u64 {
    300
}
fn default_offline_after() -> u64 {
    21_600
}
fn default_cooldown() -> u64 {
    900
}
fn default_quiet_start() -> String {
    "22:00".into()
}
fn default_quiet_end() -> String {
    "07:00".into()
}
fn default_backup_interval_hours() -> u16 {
    24
}
fn default_backup_keep_count() -> usize {
    7
}
fn default_quote_retention_days() -> u16 {
    365
}
fn default_performance_retention_days() -> u16 {
    730
}
fn default_maintenance_interval_minutes() -> u16 {
    60
}
fn default_event_log_max_bytes() -> u64 {
    2 * 1024 * 1024
}
fn default_event_log_keep_files() -> usize {
    3
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CloseBehavior {
    #[default]
    DestroyToTray,
    HideToTray,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrayLeftClickAction {
    #[default]
    OpenDashboard,
    OpenMiniMode,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MarketDataSettings {
    pub provider: String,
    pub request_timeout_seconds: u64,
    pub stale_after_seconds: u64,
    pub offline_after_seconds: u64,
    pub history_enabled: bool,
    pub refresh_on_startup: bool,
}
impl Default for MarketDataSettings {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            request_timeout_seconds: default_timeout(),
            stale_after_seconds: default_stale_after(),
            offline_after_seconds: default_offline_after(),
            history_enabled: true,
            refresh_on_startup: true,
        }
    }
}
impl MarketDataSettings {
    pub fn normalized(mut self) -> Self {
        self.provider = self.provider.trim().to_ascii_lowercase();
        self.request_timeout_seconds = self.request_timeout_seconds.clamp(2, 30);
        self.stale_after_seconds = self.stale_after_seconds.clamp(30, 86_400);
        self.offline_after_seconds = self
            .offline_after_seconds
            .max(self.stale_after_seconds.saturating_add(60))
            .min(604_800);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TraySettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_tray_symbols")]
    pub max_symbols: usize,
    #[serde(default = "default_true")]
    pub show_price: bool,
    #[serde(default = "default_true")]
    pub show_change_pct: bool,
    #[serde(default = "default_true")]
    pub show_portfolio_value: bool,
    #[serde(default = "default_true")]
    pub show_day_pnl: bool,
    #[serde(default = "default_true")]
    pub show_total_pnl: bool,
    pub left_click_action: TrayLeftClickAction,
}
impl Default for TraySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_symbols: default_max_tray_symbols(),
            show_price: true,
            show_change_pct: true,
            show_portfolio_value: true,
            show_day_pnl: true,
            show_total_pnl: true,
            left_click_action: TrayLeftClickAction::OpenDashboard,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub sound: bool,
    pub quiet_hours_enabled: bool,
    pub quiet_start: String,
    pub quiet_end: String,
    pub default_cooldown_seconds: u64,
    pub retain_history_days: u16,
}
impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            sound: true,
            quiet_hours_enabled: false,
            quiet_start: default_quiet_start(),
            quiet_end: default_quiet_end(),
            default_cooldown_seconds: default_cooldown(),
            retain_history_days: 30,
        }
    }
}
impl NotificationSettings {
    pub fn normalized(mut self) -> Self {
        if !valid_hhmm(&self.quiet_start) {
            self.quiet_start = default_quiet_start();
        }
        if !valid_hhmm(&self.quiet_end) {
            self.quiet_end = default_quiet_end();
        }
        self.default_cooldown_seconds = self.default_cooldown_seconds.min(86_400);
        self.retain_history_days = self.retain_history_days.clamp(1, 365);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ReliabilitySettings {
    #[serde(default = "default_true")]
    pub backup_enabled: bool,
    #[serde(default = "default_backup_interval_hours")]
    pub backup_interval_hours: u16,
    #[serde(default = "default_backup_keep_count")]
    pub backup_keep_count: usize,
    #[serde(default = "default_quote_retention_days")]
    pub quote_history_retention_days: u16,
    #[serde(default = "default_performance_retention_days")]
    pub performance_retention_days: u16,
    #[serde(default = "default_maintenance_interval_minutes")]
    pub maintenance_interval_minutes: u16,
    #[serde(default = "default_event_log_max_bytes")]
    pub event_log_max_bytes: u64,
    #[serde(default = "default_event_log_keep_files")]
    pub event_log_keep_files: usize,
}
impl Default for ReliabilitySettings {
    fn default() -> Self {
        Self {
            backup_enabled: true,
            backup_interval_hours: default_backup_interval_hours(),
            backup_keep_count: default_backup_keep_count(),
            quote_history_retention_days: default_quote_retention_days(),
            performance_retention_days: default_performance_retention_days(),
            maintenance_interval_minutes: default_maintenance_interval_minutes(),
            event_log_max_bytes: default_event_log_max_bytes(),
            event_log_keep_files: default_event_log_keep_files(),
        }
    }
}
impl ReliabilitySettings {
    pub fn normalized(mut self) -> Self {
        self.backup_interval_hours = self.backup_interval_hours.clamp(1, 168);
        self.backup_keep_count = self.backup_keep_count.clamp(1, 30);
        self.quote_history_retention_days = self.quote_history_retention_days.clamp(7, 3650);
        self.performance_retention_days = self.performance_retention_days.clamp(30, 3650);
        self.maintenance_interval_minutes = self.maintenance_interval_minutes.clamp(15, 1440);
        self.event_log_max_bytes = self.event_log_max_bytes.clamp(64 * 1024, 32 * 1024 * 1024);
        self.event_log_keep_files = self.event_log_keep_files.clamp(1, 10);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppSettings {
    pub launch_at_startup: bool,
    pub close_behavior: CloseBehavior,
    pub refresh: RefreshPolicy,
    pub market_data: MarketDataSettings,
    pub tray_symbols: Vec<String>,
    pub tray: TraySettings,
    pub notifications: NotificationSettings,
    pub reliability: ReliabilitySettings,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            launch_at_startup: false,
            close_behavior: CloseBehavior::DestroyToTray,
            refresh: RefreshPolicy::default(),
            market_data: MarketDataSettings::default(),
            tray_symbols: Vec::new(),
            tray: TraySettings::default(),
            notifications: NotificationSettings::default(),
            reliability: ReliabilitySettings::default(),
        }
    }
}
impl AppSettings {
    pub fn normalized(mut self) -> Self {
        self.market_data = self.market_data.normalized();
        self.notifications = self.notifications.normalized();
        self.reliability = self.reliability.normalized();
        self.tray.max_symbols = self.tray.max_symbols.clamp(1, 20);
        self.refresh.open_seconds = self.refresh.open_seconds.clamp(5, 3600);
        self.refresh.closed_seconds = self.refresh.closed_seconds.map(|v| v.clamp(60, 86_400));
        self.refresh.failure_backoff_max_seconds =
            self.refresh.failure_backoff_max_seconds.clamp(5, 3600);
        self.tray_symbols = self.effective_tray_symbols();
        self
    }
    pub fn effective_tray_symbols(&self) -> Vec<String> {
        let max = self.tray.max_symbols.clamp(1, 20);
        let mut result = Vec::new();
        for symbol in &self.tray_symbols {
            let normalized = symbol.trim().to_ascii_uppercase();
            if normalized.is_empty() || result.iter().any(|existing| existing == &normalized) {
                continue;
            }
            result.push(normalized);
            if result.len() >= max {
                break;
            }
        }
        result
    }
}

#[derive(Clone)]
pub struct SettingsState {
    inner: Arc<RwLock<AppSettings>>,
}
impl SettingsState {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            inner: Arc::new(RwLock::new(settings.normalized())),
        }
    }
    pub fn snapshot(&self) -> AppSettings {
        self.inner.read().expect("settings lock poisoned").clone()
    }
    pub fn replace(&self, settings: AppSettings) {
        *self.inner.write().expect("settings lock poisoned") = settings.normalized();
    }
}
impl Default for SettingsState {
    fn default() -> Self {
        Self::new(AppSettings::default())
    }
}

fn valid_hhmm(value: &str) -> bool {
    let mut parts = value.split(':');
    let (Some(h), Some(m), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    matches!((h.parse::<u8>(), m.parse::<u8>()), (Ok(h), Ok(m)) if h < 24 && m < 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tray_symbols_are_normalized_deduplicated_and_bounded() {
        let mut settings = AppSettings::default();
        settings.tray.max_symbols = 2;
        settings.tray_symbols = vec![
            " demotech ".into(),
            "DEMOTECH".into(),
            "demoetf".into(),
            "demobank".into(),
        ];
        assert_eq!(
            settings.effective_tray_symbols(),
            vec!["DEMOTECH", "DEMOETF"]
        );
    }
    #[test]
    fn destroy_to_tray_is_default_close_policy() {
        assert_eq!(
            AppSettings::default().close_behavior,
            CloseBehavior::DestroyToTray
        );
    }
    #[test]
    fn reliability_limits_are_bounded() {
        let r = ReliabilitySettings {
            backup_keep_count: 999,
            maintenance_interval_minutes: 1,
            ..Default::default()
        };
        let r = r.normalized();
        assert_eq!(r.backup_keep_count, 30);
        assert_eq!(r.maintenance_interval_minutes, 15);
    }
    #[test]
    fn invalid_quiet_hours_are_repaired() {
        let s = NotificationSettings {
            quiet_start: "25:99".into(),
            ..Default::default()
        };
        assert_eq!(s.normalized().quiet_start, "22:00");
    }
}
