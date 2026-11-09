use rust_decimal::prelude::ToPrimitive;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::portfolio::{DerivedPortfolio, Side, Trade};

use super::{
    AnalyticsSnapshot, BenchmarkSummary, CashFlow, CashFlowKind, HoldingContribution,
    PerformancePoint, PerformanceSnapshot, TransactionAnalytics,
};

pub fn build_analytics_snapshot(
    portfolio: &DerivedPortfolio,
    current_prices: &[(String, String, f64)],
    cash_flows: &[CashFlow],
    snapshots: &[PerformanceSnapshot],
    benchmark: Option<BenchmarkSummary>,
    trades: &[Trade],
) -> AnalyticsSnapshot {
    let mut contributions = Vec::new();
    let mut current_value = 0.0;
    let mut open_cost_basis = 0.0;
    let mut all_priced = true;

    for holding in portfolio.open_holdings() {
        let quantity = holding.quantity.to_f64().unwrap_or(0.0);
        let cost = holding.gross_cost_basis().to_f64().unwrap_or(0.0);
        open_cost_basis += cost;
        let price = current_prices
            .iter()
            .find(|(exchange, symbol, _)| {
                exchange.eq_ignore_ascii_case(&holding.instrument.exchange)
                    && symbol.eq_ignore_ascii_case(&holding.instrument.symbol)
            })
            .map(|(_, _, value)| *value);
        match price {
            Some(price) => {
                let value = price * quantity;
                current_value += value;
                contributions.push(HoldingContribution {
                    symbol: holding.instrument.symbol.clone(),
                    exchange: holding.instrument.exchange.clone(),
                    current_value: value,
                    unrealized_pnl: value - cost,
                    allocation_pct: 0.0,
                    pnl_contribution_pct: None,
                });
            }
            None => all_priced = false,
        }
    }

    let current_value = if all_priced && !contributions.is_empty() {
        Some(current_value)
    } else {
        None
    };
    if let Some(total) = current_value.filter(|v| v.abs() > f64::EPSILON) {
        for item in &mut contributions {
            item.allocation_pct = item.current_value / total * 100.0;
        }
    }
    let total_unrealized = current_value.map(|value| value - open_cost_basis);
    if let Some(total) = total_unrealized.filter(|v| v.abs() > f64::EPSILON) {
        for item in &mut contributions {
            item.pnl_contribution_pct = Some(item.unrealized_pnl / total * 100.0);
        }
    }
    contributions.sort_by(|a, b| {
        b.current_value
            .partial_cmp(&a.current_value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let realized_gross_pnl = portfolio.realized_gross_pnl.to_f64().unwrap_or(0.0);
    let recorded_fees = portfolio.fees_paid.to_f64().unwrap_or(0.0);
    let investment_income: f64 = cash_flows
        .iter()
        .filter(|flow| flow.kind.is_income())
        .map(|flow| flow.amount.to_f64().unwrap_or(0.0))
        .sum();
    let deposits: f64 = cash_flows
        .iter()
        .filter(|flow| flow.kind == CashFlowKind::Deposit)
        .map(|flow| flow.amount.to_f64().unwrap_or(0.0))
        .sum();
    let withdrawals: f64 = cash_flows
        .iter()
        .filter(|flow| flow.kind == CashFlowKind::Withdrawal)
        .map(|flow| flow.amount.to_f64().unwrap_or(0.0))
        .sum();
    let net_external_contributions = deposits - withdrawals;
    let net_total_pnl = total_unrealized
        .map(|unrealized| unrealized + realized_gross_pnl + investment_income - recorded_fees);

    let performance = performance_points(snapshots, cash_flows);
    let cash_flow_adjusted_return_pct = modified_dietz_return_pct(snapshots, cash_flows);
    let benchmark = benchmark.map(|mut item| {
        item.portfolio_excess_return_pct =
            cash_flow_adjusted_return_pct.map(|p| p - item.return_pct);
        item
    });

    let transactions = transaction_analytics(trades);

    AnalyticsSnapshot {
        current_value,
        open_cost_basis,
        unrealized_pnl: total_unrealized,
        realized_gross_pnl,
        recorded_fees,
        investment_income,
        net_total_pnl,
        cash_flow_adjusted_return_pct,
        net_external_contributions,
        deposits,
        withdrawals,
        snapshot_count: snapshots.len(),
        first_snapshot_at: snapshots.first().map(|s| s.observed_at.clone()),
        last_snapshot_at: snapshots.last().map(|s| s.observed_at.clone()),
        performance,
        contributions,
        transactions,
        benchmark,
        methodology: "Securities-only tracking. Historical return uses Modified Dietz over fully priced snapshots and explicit external deposits/withdrawals; dividends and interest are treated as investment income, not external contributions.".into(),
    }
}

pub fn modified_dietz_return_pct(
    snapshots: &[PerformanceSnapshot],
    cash_flows: &[CashFlow],
) -> Option<f64> {
    let first = snapshots.first()?;
    let last = snapshots.last()?;
    let start = parse_time(&first.observed_at)?;
    let end = parse_time(&last.observed_at)?;
    if end <= start || first.portfolio_value.abs() < f64::EPSILON {
        return None;
    }
    let total_seconds = (end - start).whole_seconds() as f64;
    if total_seconds <= 0.0 {
        return None;
    }

    let mut net_flow = 0.0;
    let mut weighted_flow = 0.0;
    for flow in cash_flows.iter().filter(|flow| flow.kind.is_external()) {
        let when = match parse_time(&flow.occurred_at) {
            Some(value) => value,
            None => continue,
        };
        if when <= start || when > end {
            continue;
        }
        let amount = flow
            .kind
            .signed_external_amount(flow.amount)
            .to_f64()
            .unwrap_or(0.0);
        let remaining = (end - when).whole_seconds().max(0) as f64;
        let weight = (remaining / total_seconds).clamp(0.0, 1.0);
        net_flow += amount;
        weighted_flow += amount * weight;
    }
    let denominator = first.portfolio_value + weighted_flow;
    if denominator.abs() < f64::EPSILON {
        return None;
    }
    Some((last.portfolio_value - first.portfolio_value - net_flow) / denominator * 100.0)
}

fn performance_points(
    snapshots: &[PerformanceSnapshot],
    cash_flows: &[CashFlow],
) -> Vec<PerformancePoint> {
    let Some(first) = snapshots.first() else {
        return Vec::new();
    };
    let Some(first_time) = parse_time(&first.observed_at) else {
        return Vec::new();
    };
    snapshots
        .iter()
        .filter_map(|snapshot| {
            let at = parse_time(&snapshot.observed_at)?;
            let cumulative_external_flow: f64 = cash_flows
                .iter()
                .filter(|flow| flow.kind.is_external())
                .filter_map(|flow| {
                    let when = parse_time(&flow.occurred_at)?;
                    if when > first_time && when <= at {
                        Some(
                            flow.kind
                                .signed_external_amount(flow.amount)
                                .to_f64()
                                .unwrap_or(0.0),
                        )
                    } else {
                        None
                    }
                })
                .sum();
            Some(PerformancePoint {
                observed_at: snapshot.observed_at.clone(),
                portfolio_value: snapshot.portfolio_value,
                cumulative_external_flow,
                gain_since_start: snapshot.portfolio_value
                    - first.portfolio_value
                    - cumulative_external_flow,
            })
        })
        .collect()
}

fn transaction_analytics(trades: &[Trade]) -> TransactionAnalytics {
    let mut out = TransactionAnalytics::default();
    for trade in trades {
        let quantity = trade.quantity.to_f64().unwrap_or(0.0);
        let price = trade.price.to_f64().unwrap_or(0.0);
        let fees = trade.fees.to_f64().unwrap_or(0.0);
        out.recorded_fees += fees;
        match trade.side {
            Side::Buy => {
                out.buy_count += 1;
                out.gross_buys += quantity * price;
            }
            Side::Sell => {
                out.sell_count += 1;
                out.gross_sells += quantity * price;
            }
        }
    }
    out
}

fn parse_time(value: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value, &Rfc3339).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::{derive_portfolio, InstrumentRef};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(v: &str) -> Decimal {
        Decimal::from_str(v).unwrap()
    }
    fn flow(kind: CashFlowKind, amount: &str, at: &str) -> CashFlow {
        CashFlow {
            id: None,
            kind,
            amount: d(amount),
            occurred_at: at.into(),
            note: None,
            external_ref: None,
        }
    }
    fn snap(value: f64, at: &str) -> PerformanceSnapshot {
        PerformanceSnapshot {
            observed_at: at.into(),
            portfolio_value: value,
            open_cost_basis: 0.0,
            realized_gross_pnl: 0.0,
            fees_paid: 0.0,
            investment_income: 0.0,
            net_external_flow: 0.0,
            source: "test".into(),
        }
    }
    fn trade(side: Side, quantity: &str, price: &str) -> Trade {
        Trade {
            id: None,
            instrument: InstrumentRef::normalized("NSE", "TEST", None),
            side,
            quantity: d(quantity),
            price: d(price),
            fees: d("0"),
            executed_at: "2026-01-01T10:00:00Z".into(),
            external_ref: None,
        }
    }

    #[test]
    fn deposit_does_not_look_like_investment_return() {
        let snapshots = vec![
            snap(1000.0, "2026-01-01T00:00:00Z"),
            snap(6100.0, "2026-01-11T00:00:00Z"),
        ];
        let flows = vec![flow(CashFlowKind::Deposit, "5000", "2026-01-06T00:00:00Z")];
        let result = modified_dietz_return_pct(&snapshots, &flows).unwrap();
        assert!(
            result > 2.0 && result < 4.0,
            "expected a small genuine return, got {result}"
        );
    }

    #[test]
    fn analytics_separates_realized_unrealized_income_and_fees() {
        let trades = vec![trade(Side::Buy, "2", "100"), trade(Side::Sell, "1", "120")];
        let portfolio = derive_portfolio(&trades).unwrap();
        let flows = vec![flow(CashFlowKind::Dividend, "5", "2026-01-10T00:00:00Z")];
        let result = build_analytics_snapshot(
            &portfolio,
            &[("NSE".into(), "TEST".into(), 130.0)],
            &flows,
            &[],
            None,
            &trades,
        );
        assert_eq!(result.realized_gross_pnl, 20.0);
        assert_eq!(result.unrealized_pnl, Some(30.0));
        assert_eq!(result.investment_income, 5.0);
        assert_eq!(result.net_total_pnl, Some(55.0));
    }
}
