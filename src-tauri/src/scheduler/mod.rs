use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::{OffsetDateTime, UtcOffset, Weekday};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketSessionState {
    Open,
    Closed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefreshPolicy {
    pub open_seconds: u64,
    pub closed_seconds: Option<u64>,
    pub failure_backoff_max_seconds: u64,
}

impl Default for RefreshPolicy {
    fn default() -> Self {
        Self {
            open_seconds: 30,
            closed_seconds: None,
            failure_backoff_max_seconds: 300,
        }
    }
}

impl RefreshPolicy {
    pub fn interval(&self, state: MarketSessionState) -> Option<Duration> {
        match state {
            MarketSessionState::Open => Some(Duration::from_secs(self.open_seconds.max(5))),
            MarketSessionState::Closed => self
                .closed_seconds
                .map(|seconds| Duration::from_secs(seconds.max(60))),
            MarketSessionState::Unknown => Some(Duration::from_secs(
                self.failure_backoff_max_seconds.clamp(5, 60),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackoffState {
    pub consecutive_failures: u32,
}

impl BackoffState {
    pub fn success(&mut self) {
        self.consecutive_failures = 0;
    }
    pub fn failure(&mut self) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
    }

    pub fn delay(&self, base_seconds: u64, max_seconds: u64) -> Duration {
        let shift = self.consecutive_failures.saturating_sub(1).min(10);
        let multiplier = 1_u64 << shift;
        Duration::from_secs(
            base_seconds
                .max(1)
                .saturating_mul(multiplier)
                .min(max_seconds.max(1)),
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct WakeDetector {
    pub last_seen_unix_ms: Option<i64>,
    pub threshold_ms: i64,
}

impl WakeDetector {
    pub fn new(threshold_seconds: u64) -> Self {
        Self {
            last_seen_unix_ms: None,
            threshold_ms: threshold_seconds
                .max(60)
                .saturating_mul(1000)
                .min(i64::MAX as u64) as i64,
        }
    }

    pub fn observe(&mut self, now_unix_ms: i64) -> bool {
        let woke = self
            .last_seen_unix_ms
            .map(|last| now_unix_ms.saturating_sub(last) > self.threshold_ms)
            .unwrap_or(false);
        self.last_seen_unix_ms = Some(now_unix_ms);
        woke
    }
}

/// Local clock estimate for normal NSE trading hours (09:15–15:30 IST, Monday–Friday).
/// Exchange holidays and special sessions are deliberately not inferred here; a provider/calendar
/// adapter can override this estimate later. This prevents the UI from claiming authoritative
/// exchange status from a weekday clock alone.
pub fn estimate_nse_session(now_utc: OffsetDateTime) -> MarketSessionState {
    let ist = UtcOffset::from_hms(5, 30, 0).expect("valid IST offset");
    let local = now_utc.to_offset(ist);
    if matches!(local.weekday(), Weekday::Saturday | Weekday::Sunday) {
        return MarketSessionState::Closed;
    }
    let minutes = u16::from(local.hour()) * 60 + u16::from(local.minute());
    let open = 9 * 60 + 15;
    let close = 15 * 60 + 30;
    if minutes >= open && minutes <= close {
        MarketSessionState::Open
    } else {
        MarketSessionState::Closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_detector_flags_large_clock_gaps() {
        let mut detector = WakeDetector::new(120);
        assert!(!detector.observe(1_000_000));
        assert!(!detector.observe(1_060_000));
        assert!(detector.observe(1_181_000));
    }

    #[test]
    fn exponential_backoff_is_bounded() {
        let mut state = BackoffState::default();
        state.failure();
        assert_eq!(state.delay(5, 60), Duration::from_secs(5));
        state.failure();
        assert_eq!(state.delay(5, 60), Duration::from_secs(10));
        for _ in 0..10 {
            state.failure();
        }
        assert_eq!(state.delay(5, 60), Duration::from_secs(60));
        state.success();
        assert_eq!(state.consecutive_failures, 0);
    }
}
