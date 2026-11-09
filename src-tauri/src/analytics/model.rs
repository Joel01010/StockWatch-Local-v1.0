use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CashFlowKind {
    Deposit,
    Withdrawal,
    Dividend,
    Interest,
}

impl CashFlowKind {
    pub fn is_external(self) -> bool {
        matches!(self, Self::Deposit | Self::Withdrawal)
    }

    pub fn signed_external_amount(self, amount: Decimal) -> Decimal {
        match self {
            Self::Deposit => amount,
            Self::Withdrawal => -amount,
            _ => Decimal::ZERO,
        }
    }

    pub fn is_income(self) -> bool {
        matches!(self, Self::Dividend | Self::Interest)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CashFlow {
    pub id: Option<i64>,
    pub kind: CashFlowKind,
    pub amount: Decimal,
    pub occurred_at: String,
    pub note: Option<String>,
    pub external_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSnapshot {
    pub observed_at: String,
    pub portfolio_value: f64,
    pub open_cost_basis: f64,
    pub realized_gross_pnl: f64,
    pub fees_paid: f64,
    pub investment_income: f64,
    pub net_external_flow: f64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PerformancePoint {
    pub observed_at: String,
    pub portfolio_value: f64,
    pub cumulative_external_flow: f64,
    pub gain_since_start: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HoldingContribution {
    pub symbol: String,
    pub exchange: String,
    pub current_value: f64,
    pub unrealized_pnl: f64,
    pub allocation_pct: f64,
    pub pnl_contribution_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransactionAnalytics {
    pub buy_count: usize,
    pub sell_count: usize,
    pub gross_buys: f64,
    pub gross_sells: f64,
    pub recorded_fees: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkSummary {
    pub exchange: String,
    pub symbol: String,
    pub start_price: f64,
    pub end_price: f64,
    pub return_pct: f64,
    pub portfolio_excess_return_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsSnapshot {
    pub current_value: Option<f64>,
    pub open_cost_basis: f64,
    pub unrealized_pnl: Option<f64>,
    pub realized_gross_pnl: f64,
    pub recorded_fees: f64,
    pub investment_income: f64,
    pub net_total_pnl: Option<f64>,
    pub cash_flow_adjusted_return_pct: Option<f64>,
    pub net_external_contributions: f64,
    pub deposits: f64,
    pub withdrawals: f64,
    pub snapshot_count: usize,
    pub first_snapshot_at: Option<String>,
    pub last_snapshot_at: Option<String>,
    pub performance: Vec<PerformancePoint>,
    pub contributions: Vec<HoldingContribution>,
    pub transactions: TransactionAnalytics,
    pub benchmark: Option<BenchmarkSummary>,
    pub methodology: String,
}
