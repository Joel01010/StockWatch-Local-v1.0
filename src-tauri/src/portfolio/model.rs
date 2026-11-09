use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstrumentRef {
    pub exchange: String,
    pub symbol: String,
    pub name: Option<String>,
}

impl InstrumentRef {
    pub fn normalized(
        exchange: impl Into<String>,
        symbol: impl Into<String>,
        name: Option<String>,
    ) -> Self {
        Self {
            exchange: exchange.into().trim().to_ascii_uppercase(),
            symbol: symbol.into().trim().to_ascii_uppercase(),
            name: name
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.exchange, self.symbol)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trade {
    pub id: Option<i64>,
    pub instrument: InstrumentRef,
    pub side: Side,
    pub quantity: Decimal,
    pub price: Decimal,
    pub fees: Decimal,
    /// RFC3339 timestamp. The engine parses this to an absolute instant before ordering.
    pub executed_at: String,
    pub external_ref: Option<String>,
}

impl Trade {
    pub fn gross_value(&self) -> Decimal {
        self.quantity * self.price
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Holding {
    pub instrument: InstrumentRef,
    pub quantity: Decimal,
    /// Weighted-average acquisition price, excluding transaction fees.
    pub average_price: Decimal,
    pub realized_gross_pnl: Decimal,
    pub fees_paid: Decimal,
    pub buy_count: u64,
    pub sell_count: u64,
}

impl Holding {
    pub fn gross_cost_basis(&self) -> Decimal {
        self.quantity * self.average_price
    }

    pub fn current_value(&self, last_price: Decimal) -> Decimal {
        self.quantity * last_price
    }

    pub fn unrealized_gross_pnl(&self, last_price: Decimal) -> Decimal {
        self.current_value(last_price) - self.gross_cost_basis()
    }

    pub fn return_pct(&self, last_price: Decimal) -> Option<Decimal> {
        let invested = self.gross_cost_basis();
        if invested.is_zero() {
            None
        } else {
            Some(self.unrealized_gross_pnl(last_price) / invested * Decimal::new(100, 0))
        }
    }

    pub fn is_open(&self) -> bool {
        self.quantity > Decimal::ZERO
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DerivedPortfolio {
    pub positions: Vec<Holding>,
    pub realized_gross_pnl: Decimal,
    pub fees_paid: Decimal,
}

impl DerivedPortfolio {
    pub fn open_holdings(&self) -> impl Iterator<Item = &Holding> {
        self.positions.iter().filter(|position| position.is_open())
    }

    pub fn total_open_cost_basis(&self) -> Decimal {
        self.open_holdings().map(Holding::gross_cost_basis).sum()
    }

    /// Realized P&L after all recorded transaction fees. Buy fees are charged immediately,
    /// so this deliberately differs from broker tax-report cost-basis accounting.
    pub fn net_realized_after_all_fees(&self) -> Decimal {
        self.realized_gross_pnl - self.fees_paid
    }
}
