use std::collections::BTreeMap;

use rust_decimal::Decimal;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::model::{DerivedPortfolio, Holding, InstrumentRef, Side, Trade};

#[derive(Debug, Error, PartialEq)]
pub enum PortfolioError {
    #[error("trade {index} has an empty exchange or symbol")]
    InvalidInstrument { index: usize },
    #[error("trade {index} has a non-positive quantity")]
    InvalidQuantity { index: usize },
    #[error("trade {index} has a negative price")]
    InvalidPrice { index: usize },
    #[error("trade {index} has negative fees")]
    InvalidFees { index: usize },
    #[error("trade {index} has an invalid RFC3339 execution timestamp")]
    InvalidTimestamp { index: usize },
    #[error("sell would oversell {instrument}: attempted {attempted}, available {available}")]
    Oversell {
        instrument: String,
        attempted: Decimal,
        available: Decimal,
    },
}

#[derive(Debug, Clone)]
struct PositionAccumulator {
    instrument: InstrumentRef,
    quantity: Decimal,
    average_price: Decimal,
    realized_gross_pnl: Decimal,
    fees_paid: Decimal,
    buy_count: u64,
    sell_count: u64,
}

impl PositionAccumulator {
    fn new(instrument: InstrumentRef) -> Self {
        Self {
            instrument,
            quantity: Decimal::ZERO,
            average_price: Decimal::ZERO,
            realized_gross_pnl: Decimal::ZERO,
            fees_paid: Decimal::ZERO,
            buy_count: 0,
            sell_count: 0,
        }
    }

    fn into_holding(self) -> Holding {
        Holding {
            instrument: self.instrument,
            quantity: self.quantity,
            average_price: self.average_price,
            realized_gross_pnl: self.realized_gross_pnl,
            fees_paid: self.fees_paid,
            buy_count: self.buy_count,
            sell_count: self.sell_count,
        }
    }
}

pub fn derive_portfolio(trades: &[Trade]) -> Result<DerivedPortfolio, PortfolioError> {
    let mut ordered: Vec<(usize, &Trade, OffsetDateTime)> = Vec::with_capacity(trades.len());
    for (index, trade) in trades.iter().enumerate() {
        let timestamp = validate_trade(index, trade)?;
        ordered.push((index, trade, timestamp));
    }

    ordered.sort_by(
        |(left_index, left, left_time), (right_index, right, right_time)| {
            left_time
                .cmp(right_time)
                .then_with(|| {
                    left.id
                        .unwrap_or(i64::MAX)
                        .cmp(&right.id.unwrap_or(i64::MAX))
                })
                .then_with(|| left_index.cmp(right_index))
        },
    );

    let mut positions: BTreeMap<String, PositionAccumulator> = BTreeMap::new();
    let mut total_realized = Decimal::ZERO;
    let mut total_fees = Decimal::ZERO;

    for (_, trade, _) in ordered {
        let key = trade.instrument.key();
        let position = positions
            .entry(key.clone())
            .or_insert_with(|| PositionAccumulator::new(trade.instrument.clone()));

        total_fees += trade.fees;
        position.fees_paid += trade.fees;

        match trade.side {
            Side::Buy => {
                let previous_cost = position.quantity * position.average_price;
                let added_cost = trade.quantity * trade.price;
                let new_quantity = position.quantity + trade.quantity;
                position.average_price = if new_quantity.is_zero() {
                    Decimal::ZERO
                } else {
                    (previous_cost + added_cost) / new_quantity
                };
                position.quantity = new_quantity;
                position.buy_count += 1;
            }
            Side::Sell => {
                if trade.quantity > position.quantity {
                    return Err(PortfolioError::Oversell {
                        instrument: key,
                        attempted: trade.quantity,
                        available: position.quantity,
                    });
                }

                let realized = trade.quantity * (trade.price - position.average_price);
                position.realized_gross_pnl += realized;
                total_realized += realized;
                position.quantity -= trade.quantity;
                position.sell_count += 1;

                if position.quantity.is_zero() {
                    position.quantity = Decimal::ZERO;
                    position.average_price = Decimal::ZERO;
                }
            }
        }
    }

    Ok(DerivedPortfolio {
        positions: positions
            .into_values()
            .map(PositionAccumulator::into_holding)
            .collect(),
        realized_gross_pnl: total_realized,
        fees_paid: total_fees,
    })
}

fn validate_trade(index: usize, trade: &Trade) -> Result<OffsetDateTime, PortfolioError> {
    if trade.instrument.exchange.trim().is_empty() || trade.instrument.symbol.trim().is_empty() {
        return Err(PortfolioError::InvalidInstrument { index });
    }
    if trade.quantity <= Decimal::ZERO {
        return Err(PortfolioError::InvalidQuantity { index });
    }
    if trade.price < Decimal::ZERO {
        return Err(PortfolioError::InvalidPrice { index });
    }
    if trade.fees < Decimal::ZERO {
        return Err(PortfolioError::InvalidFees { index });
    }
    OffsetDateTime::parse(trade.executed_at.trim(), &Rfc3339)
        .map_err(|_| PortfolioError::InvalidTimestamp { index })
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    use super::*;

    fn d(value: &str) -> Decimal {
        Decimal::from_str(value).unwrap()
    }

    fn trade(side: Side, qty: &str, price: &str, fees: &str, when: &str) -> Trade {
        Trade {
            id: None,
            instrument: InstrumentRef::normalized("NSE", "TEST", Some("Test Ltd".into())),
            side,
            quantity: d(qty),
            price: d(price),
            fees: d(fees),
            executed_at: when.into(),
            external_ref: None,
        }
    }

    #[test]
    fn weighted_average_uses_only_buys() {
        let result = derive_portfolio(&[
            trade(Side::Buy, "2", "100", "1", "2026-01-01T10:00:00Z"),
            trade(Side::Buy, "1", "130", "1", "2026-01-02T10:00:00Z"),
        ])
        .unwrap();
        let p = &result.positions[0];
        assert_eq!(p.quantity, d("3"));
        assert_eq!(p.average_price, d("110"));
        assert_eq!(p.gross_cost_basis(), d("330"));
        assert_eq!(result.fees_paid, d("2"));
    }

    #[test]
    fn partial_sell_realizes_against_weighted_average() {
        let result = derive_portfolio(&[
            trade(Side::Buy, "2", "100", "1", "2026-01-01T10:00:00Z"),
            trade(Side::Buy, "2", "120", "1", "2026-01-02T10:00:00Z"),
            trade(Side::Sell, "1", "140", "2", "2026-01-03T10:00:00Z"),
        ])
        .unwrap();
        let p = &result.positions[0];
        assert_eq!(p.quantity, d("3"));
        assert_eq!(p.average_price, d("110"));
        assert_eq!(p.realized_gross_pnl, d("30"));
        assert_eq!(result.net_realized_after_all_fees(), d("26"));
    }

    #[test]
    fn closing_a_position_resets_average_price() {
        let result = derive_portfolio(&[
            trade(Side::Buy, "1", "100", "0", "2026-01-01T10:00:00Z"),
            trade(Side::Sell, "1", "120", "0", "2026-01-02T10:00:00Z"),
            trade(Side::Buy, "1", "75", "0", "2026-01-03T10:00:00Z"),
        ])
        .unwrap();
        let p = &result.positions[0];
        assert_eq!(p.quantity, d("1"));
        assert_eq!(p.average_price, d("75"));
        assert_eq!(p.realized_gross_pnl, d("20"));
    }

    #[test]
    fn rejects_oversell() {
        let error = derive_portfolio(&[
            trade(Side::Buy, "1", "100", "0", "2026-01-01T10:00:00Z"),
            trade(Side::Sell, "2", "120", "0", "2026-01-02T10:00:00Z"),
        ])
        .unwrap_err();
        assert!(matches!(error, PortfolioError::Oversell { .. }));
    }

    #[test]
    fn input_order_does_not_change_timestamp_ordered_result() {
        let buy = trade(Side::Buy, "1", "100", "0", "2026-01-01T10:00:00Z");
        let sell = trade(Side::Sell, "1", "110", "0", "2026-01-02T10:00:00Z");
        let result = derive_portfolio(&[sell, buy]).unwrap();
        assert_eq!(result.realized_gross_pnl, d("10"));
    }

    #[test]
    fn equivalent_offsets_are_sorted_by_actual_instant_not_text() {
        let buy = trade(Side::Buy, "1", "100", "0", "2026-01-01T10:00:00+05:30");
        let sell = trade(Side::Sell, "1", "110", "0", "2026-01-01T05:00:01Z");
        let result = derive_portfolio(&[sell, buy]).unwrap();
        assert_eq!(result.realized_gross_pnl, d("10"));
    }
}
