use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::{market::QuoteFreshness, scheduler::MarketSessionState};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeHolding {
    pub symbol: String,
    pub exchange: String,
    pub price: Option<f64>,
    pub change_pct: Option<f64>,
    pub freshness: QuoteFreshness,
    pub source: Option<String>,
    pub observed_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeSnapshot {
    pub market_session: MarketSessionState,
    pub provider_id: Option<String>,
    pub last_refresh_unix_ms: Option<i64>,
    pub refresh_in_flight: bool,
    pub last_refresh_error: Option<String>,
    pub active_alert_count: usize,
    pub holdings: Vec<RuntimeHolding>,
    pub portfolio_value: Option<f64>,
    pub day_pnl: Option<f64>,
    pub total_pnl: Option<f64>,
    pub refresh_success_count: u64,
    pub refresh_error_count: u64,
    pub last_refresh_duration_ms: Option<u64>,
    pub wake_recovery_count: u64,
}

impl Default for RuntimeSnapshot {
    fn default() -> Self {
        Self {
            market_session: MarketSessionState::Unknown,
            provider_id: None,
            last_refresh_unix_ms: None,
            refresh_in_flight: false,
            last_refresh_error: None,
            active_alert_count: 0,
            holdings: Vec::new(),
            portfolio_value: None,
            day_pnl: None,
            total_pnl: None,
            refresh_success_count: 0,
            refresh_error_count: 0,
            last_refresh_duration_ms: None,
            wake_recovery_count: 0,
        }
    }
}

#[derive(Clone, Default)]
pub struct RuntimeState {
    inner: Arc<RwLock<RuntimeSnapshot>>,
}

impl RuntimeState {
    pub fn snapshot(&self) -> RuntimeSnapshot {
        self.inner
            .read()
            .expect("runtime state lock poisoned")
            .clone()
    }

    pub fn replace(&self, snapshot: RuntimeSnapshot) {
        *self.inner.write().expect("runtime state lock poisoned") = snapshot;
    }

    pub fn update<F>(&self, update: F)
    where
        F: FnOnce(&mut RuntimeSnapshot),
    {
        let mut guard = self.inner.write().expect("runtime state lock poisoned");
        update(&mut guard);
    }

    pub fn try_begin_refresh(&self) -> bool {
        let mut guard = self.inner.write().expect("runtime state lock poisoned");
        if guard.refresh_in_flight {
            return false;
        }
        guard.refresh_in_flight = true;
        guard.last_refresh_error = None;
        true
    }

    pub fn finish_refresh_ok(&self, observed_unix_ms: i64) {
        self.update(|state| {
            state.refresh_in_flight = false;
            state.last_refresh_unix_ms = Some(observed_unix_ms);
            state.last_refresh_error = None;
            state.refresh_success_count = state.refresh_success_count.saturating_add(1);
        });
    }

    pub fn finish_refresh_error(&self, message: impl Into<String>) {
        let message = message.into();
        self.update(move |state| {
            state.refresh_in_flight = false;
            state.last_refresh_error = Some(message);
            state.refresh_error_count = state.refresh_error_count.saturating_add(1);
        });
    }

    pub fn record_refresh_duration(&self, duration_ms: u64) {
        self.update(|state| state.last_refresh_duration_ms = Some(duration_ms));
    }

    pub fn note_wake_recovery(&self) {
        self.update(|state| {
            state.wake_recovery_count = state.wake_recovery_count.saturating_add(1)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_is_shared_between_clones() {
        let state = RuntimeState::default();
        let clone = state.clone();
        state.update(|s| s.active_alert_count = 3);
        assert_eq!(clone.snapshot().active_alert_count, 3);
    }

    #[test]
    fn refresh_gate_rejects_duplicates_and_recovers_after_error() {
        let state = RuntimeState::default();
        assert!(state.try_begin_refresh());
        assert!(!state.try_begin_refresh());
        state.finish_refresh_error("provider not configured");
        assert!(!state.snapshot().refresh_in_flight);
        assert!(state.try_begin_refresh());
    }
}
