use super::QuoteFreshness;

#[derive(Debug, Clone, Copy)]
pub struct FreshnessPolicy {
    pub stale_after_seconds: u64,
    pub offline_after_seconds: u64,
}

impl Default for FreshnessPolicy {
    fn default() -> Self {
        Self {
            stale_after_seconds: 300,
            offline_after_seconds: 21_600,
        }
    }
}

impl FreshnessPolicy {
    pub fn classify(&self, baseline: QuoteFreshness, age_seconds: u64) -> QuoteFreshness {
        if age_seconds >= self.offline_after_seconds {
            QuoteFreshness::Offline
        } else if age_seconds >= self.stale_after_seconds {
            QuoteFreshness::Stale
        } else {
            baseline
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delayed_quotes_age_to_stale_then_offline() {
        let p = FreshnessPolicy {
            stale_after_seconds: 60,
            offline_after_seconds: 600,
        };
        assert_eq!(
            p.classify(QuoteFreshness::Delayed, 10),
            QuoteFreshness::Delayed
        );
        assert_eq!(
            p.classify(QuoteFreshness::Delayed, 60),
            QuoteFreshness::Stale
        );
        assert_eq!(
            p.classify(QuoteFreshness::Delayed, 600),
            QuoteFreshness::Offline
        );
    }
}
