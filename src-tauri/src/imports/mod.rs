use std::{io::Read, str::FromStr};

use rust_decimal::Decimal;
use serde::Deserialize;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::portfolio::{InstrumentRef, Side, Trade};

#[derive(Debug, Clone, PartialEq)]
pub struct OpeningHolding {
    pub instrument: InstrumentRef,
    pub quantity: Decimal,
    /// Broker-displayed average price. It may be rounded for presentation.
    pub average_price: Decimal,
    /// Optional exact broker-reported invested amount. When present, this is used
    /// to reconstruct the opening cost basis and avoid drift from a rounded average.
    pub invested_amount: Option<Decimal>,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("row {row}: {message}")]
    InvalidRow { row: usize, message: String },
}

#[derive(Debug, Deserialize)]
struct TransactionCsvRecord {
    exchange: String,
    symbol: String,
    #[serde(default)]
    name: Option<String>,
    side: String,
    quantity: String,
    price: String,
    #[serde(default = "zero_string")]
    fees: String,
    executed_at: String,
    #[serde(default)]
    external_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HoldingCsvRecord {
    exchange: String,
    symbol: String,
    #[serde(default)]
    name: Option<String>,
    quantity: String,
    average_price: String,
    #[serde(default)]
    invested_amount: Option<String>,
}

fn zero_string() -> String {
    "0".into()
}

pub fn parse_transactions_csv<R: Read>(reader: R) -> Result<Vec<Trade>, ImportError> {
    let mut csv = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);
    let mut trades = Vec::new();

    for (index, row) in csv.deserialize::<TransactionCsvRecord>().enumerate() {
        let row_number = index + 2;
        let row = row?;
        let instrument = validate_instrument(row_number, row.exchange, row.symbol, row.name)?;
        let side = match row.side.trim().to_ascii_lowercase().as_str() {
            "buy" | "b" => Side::Buy,
            "sell" | "s" => Side::Sell,
            _ => return Err(invalid(row_number, "side must be buy or sell")),
        };
        let quantity = positive_decimal(row_number, "quantity", &row.quantity)?;
        let price = non_negative_decimal(row_number, "price", &row.price)?;
        let fees = if row.fees.trim().is_empty() {
            Decimal::ZERO
        } else {
            non_negative_decimal(row_number, "fees", &row.fees)?
        };
        validate_rfc3339(row_number, &row.executed_at)?;

        trades.push(Trade {
            id: None,
            instrument,
            side,
            quantity,
            price,
            fees,
            executed_at: row.executed_at.trim().to_string(),
            external_ref: normalize_optional(row.external_ref),
        });
    }

    Ok(trades)
}

pub fn parse_holdings_csv<R: Read>(reader: R) -> Result<Vec<OpeningHolding>, ImportError> {
    let mut csv = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);
    let mut holdings = Vec::new();

    for (index, row) in csv.deserialize::<HoldingCsvRecord>().enumerate() {
        let row_number = index + 2;
        let row = row?;
        let instrument = validate_instrument(row_number, row.exchange, row.symbol, row.name)?;
        let quantity = positive_decimal(row_number, "quantity", &row.quantity)?;
        let average_price = non_negative_decimal(row_number, "average_price", &row.average_price)?;
        let invested_amount = match row.invested_amount.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => Some(non_negative_decimal(row_number, "invested_amount", value)?),
        };
        holdings.push(OpeningHolding {
            instrument,
            quantity,
            average_price,
            invested_amount,
        });
    }

    Ok(holdings)
}

pub fn opening_holdings_to_trades(
    holdings: &[OpeningHolding],
    observed_at: &str,
    import_batch: &str,
) -> Result<Vec<Trade>, ImportError> {
    validate_rfc3339(1, observed_at)?;
    Ok(holdings
        .iter()
        .enumerate()
        .map(|(index, holding)| Trade {
            id: None,
            instrument: holding.instrument.clone(),
            side: Side::Buy,
            quantity: holding.quantity,
            price: holding
                .invested_amount
                .map(|amount| amount / holding.quantity)
                .unwrap_or(holding.average_price),
            fees: Decimal::ZERO,
            executed_at: observed_at.to_string(),
            external_ref: Some(format!(
                "opening:{}:{}:{}:{}",
                import_batch,
                holding.instrument.exchange,
                holding.instrument.symbol,
                index + 1
            )),
        })
        .collect())
}

fn validate_instrument(
    row: usize,
    exchange: String,
    symbol: String,
    name: Option<String>,
) -> Result<InstrumentRef, ImportError> {
    let instrument = InstrumentRef::normalized(exchange, symbol, name);
    if instrument.exchange.is_empty() || instrument.symbol.is_empty() {
        Err(invalid(row, "exchange and symbol are required"))
    } else {
        Ok(instrument)
    }
}

fn positive_decimal(row: usize, field: &str, value: &str) -> Result<Decimal, ImportError> {
    let parsed = decimal(row, field, value)?;
    if parsed <= Decimal::ZERO {
        Err(invalid(row, format!("{field} must be greater than zero")))
    } else {
        Ok(parsed)
    }
}

fn non_negative_decimal(row: usize, field: &str, value: &str) -> Result<Decimal, ImportError> {
    let parsed = decimal(row, field, value)?;
    if parsed < Decimal::ZERO {
        Err(invalid(row, format!("{field} cannot be negative")))
    } else {
        Ok(parsed)
    }
}

fn decimal(row: usize, field: &str, value: &str) -> Result<Decimal, ImportError> {
    Decimal::from_str(value.trim())
        .map_err(|_| invalid(row, format!("{field} is not a valid decimal number")))
}

fn validate_rfc3339(row: usize, value: &str) -> Result<(), ImportError> {
    OffsetDateTime::parse(value.trim(), &Rfc3339)
        .map(|_| ())
        .map_err(|_| invalid(row, "executed_at must be an RFC3339 timestamp"))
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn invalid(row: usize, message: impl Into<String>) -> ImportError {
    ImportError::InvalidRow {
        row,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_transaction_csv_and_normalizes_symbol() {
        let input = "exchange,symbol,name,side,quantity,price,fees,executed_at,external_ref\n\
                     nse, demotech ,Demo Technology,buy,4,200.00,0.10,2026-11-01T10:00:00Z,abc\n";
        let rows = parse_transactions_csv(input.as_bytes()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].instrument.exchange, "NSE");
        assert_eq!(rows[0].instrument.symbol, "DEMOTECH");
    }

    #[test]
    fn holdings_conversion_uses_stable_external_refs() {
        let input = "exchange,symbol,quantity,average_price,invested_amount\nNSE,DEMOETF,3,100.00,300.00\n";
        let holdings = parse_holdings_csv(input.as_bytes()).unwrap();
        let trades =
            opening_holdings_to_trades(&holdings, "2026-11-08T00:00:00Z", "batch-a").unwrap();
        assert_eq!(
            trades[0].external_ref.as_deref(),
            Some("opening:batch-a:NSE:DEMOETF:1")
        );
    }

    #[test]
    fn exact_invested_amount_overrides_rounded_display_average() {
        let input =
            "exchange,symbol,quantity,average_price,invested_amount\nNSE,DEMOTECH,4,200.00,800.01\n";
        let holdings = parse_holdings_csv(input.as_bytes()).unwrap();
        let trades =
            opening_holdings_to_trades(&holdings, "2026-11-08T00:00:00Z", "batch-rounded").unwrap();
        assert_eq!(
            trades[0].gross_value(),
            Decimal::from_str("800.01").unwrap()
        );
    }

    #[test]
    fn rejects_negative_fees() {
        let input = "exchange,symbol,side,quantity,price,fees,executed_at\nNSE,X,buy,1,100,-1,2026-11-01T10:00:00Z\n";
        assert!(parse_transactions_csv(input.as_bytes()).is_err());
    }

    #[test]
    fn rejects_non_rfc3339_timestamp() {
        let input = "exchange,symbol,side,quantity,price,fees,executed_at\nNSE,X,buy,1,100,0,08-11-2026 10:00\n";
        assert!(parse_transactions_csv(input.as_bytes()).is_err());
    }
}
