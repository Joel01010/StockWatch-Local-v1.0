use std::{collections::BTreeMap, path::Path, str::FromStr};

use rusqlite::{params, Connection, OptionalExtension, Transaction as SqlTransaction};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::Serialize;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    alerts::{AlertEvent, AlertRepeatMode, AlertRule},
    analytics::{CashFlow, CashFlowKind, PerformanceSnapshot},
    market::{MarketInstrument, Quote, QuoteFreshness},
    portfolio::{derive_portfolio, DerivedPortfolio, InstrumentRef, PortfolioError, Side, Trade},
    settings::AppSettings,
};

use super::{
    MIGRATION_0001, MIGRATION_0002, MIGRATION_0003, MIGRATION_0004, MIGRATION_0005, MIGRATION_0006,
    SCHEMA_VERSION,
};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("portfolio calculation failed: {0}")]
    Portfolio(#[from] PortfolioError),
    #[error("database contains an invalid decimal value in {field}: {value}")]
    InvalidDecimal { field: &'static str, value: String },
    #[error("database contains invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("database contains invalid quote freshness: {0}")]
    InvalidFreshness(String),
    #[error("database contains invalid JSON: {0}")]
    InvalidJson(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsertReport {
    pub requested: usize,
    pub inserted: usize,
    pub skipped_duplicates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderStateRecord {
    pub provider_id: String,
    pub consecutive_failures: u32,
    pub last_success_at: Option<String>,
    pub last_error: Option<String>,
    pub next_retry_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct HistoryPruneReport {
    pub quote_history: usize,
    pub performance_snapshots: usize,
    pub alert_events: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlertEventRecord {
    pub id: i64,
    pub alert_id: String,
    pub fired_at: String,
    pub metric_value: Option<f64>,
    pub title: String,
    pub body: String,
    pub delivery_status: String,
    pub delivery_error: Option<String>,
}

pub struct PortfolioRepository {
    connection: Connection,
}

impl PortfolioRepository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        Self::from_connection(connection)
    }

    fn from_connection(mut connection: Connection) -> Result<Self, StorageError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "busy_timeout", 5000_i64)?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA wal_autocheckpoint=1000;",
        )?;

        let has_schema_meta: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_meta')",
            [],
            |row| row.get(0),
        )?;
        let current_schema = if has_schema_meta {
            connection
                .query_row(
                    "SELECT value FROM schema_meta WHERE key='schema_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0)
        } else {
            0
        };
        if current_schema > SCHEMA_VERSION {
            return Err(StorageError::InvalidJson(format!(
                "database schema {current_schema} is newer than supported schema {SCHEMA_VERSION}"
            )));
        }
        if current_schema < SCHEMA_VERSION {
            let migrations: &[(i64, &str)] = &[
                (1, MIGRATION_0001),
                (2, MIGRATION_0002),
                (3, MIGRATION_0003),
                (4, MIGRATION_0004),
                (5, MIGRATION_0005),
                (6, MIGRATION_0006),
            ];
            let tx = connection.transaction()?;
            for (version, sql) in migrations {
                if *version > current_schema {
                    tx.execute_batch(sql)?;
                }
            }
            tx.commit()?;
        }
        let repo = Self { connection };
        let actual = repo.schema_version()?;
        if actual != SCHEMA_VERSION {
            return Err(StorageError::InvalidJson(format!(
                "migration ended at schema {actual}, expected {SCHEMA_VERSION}"
            )));
        }
        Ok(repo)
    }

    pub fn schema_version(&self) -> Result<i64, StorageError> {
        let value: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value.and_then(|v| v.parse().ok()).unwrap_or(0))
    }

    pub fn insert_trades_atomic(&mut self, trades: &[Trade]) -> Result<InsertReport, StorageError> {
        let transaction = self.connection.transaction()?;
        let mut inserted = 0usize;
        for trade in trades {
            let instrument_id = upsert_instrument(&transaction, &trade.instrument)?;
            inserted += insert_trade(&transaction, instrument_id, trade)? as usize;
        }
        derive_portfolio(&load_trades(&transaction)?)?;
        transaction.commit()?;
        Ok(InsertReport {
            requested: trades.len(),
            inserted,
            skipped_duplicates: trades.len().saturating_sub(inserted),
        })
    }

    pub fn list_trades(&self) -> Result<Vec<Trade>, StorageError> {
        load_trades(&self.connection)
    }
    pub fn derive_portfolio(&self) -> Result<DerivedPortfolio, StorageError> {
        Ok(derive_portfolio(&self.list_trades()?)?)
    }
    pub fn transaction_count(&self) -> Result<i64, StorageError> {
        Ok(self
            .connection
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))?)
    }
    pub fn expected_schema_version() -> i64 {
        SCHEMA_VERSION
    }

    /// Union of open holdings and watchlist instruments. Symbols are normalized and deduplicated.
    pub fn quote_targets(&self, provider_id: &str) -> Result<Vec<MarketInstrument>, StorageError> {
        let mut targets: BTreeMap<String, MarketInstrument> = BTreeMap::new();
        for holding in self.derive_portfolio()?.open_holdings() {
            let instrument = MarketInstrument::normalized(
                holding.instrument.exchange.clone(),
                holding.instrument.symbol.clone(),
                holding.instrument.name.clone(),
            );
            targets.insert(instrument.key(), instrument);
        }

        let mut statement = self.connection.prepare(
            "SELECT i.exchange, i.symbol, i.name FROM watchlist_items wi \
             JOIN instruments i ON i.id = wi.instrument_id ORDER BY wi.watchlist_id, wi.position, i.symbol",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in rows {
            let (exchange, symbol, name) = row?;
            let instrument = MarketInstrument::normalized(exchange, symbol, name);
            targets.insert(instrument.key(), instrument);
        }

        // Enabled symbol-based alerts are quote consumers too. This lets a user track an
        // alert-only instrument without first creating a holding or watchlist entry.
        let mut alert_statement = self.connection.prepare(
            "SELECT DISTINCT i.exchange, i.symbol, i.name FROM alerts a \
             JOIN instruments i ON i.id = a.instrument_id WHERE a.enabled = 1 ORDER BY i.exchange, i.symbol",
        )?;
        let alert_rows = alert_statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in alert_rows {
            let (exchange, symbol, name) = row?;
            let instrument = MarketInstrument::normalized(exchange, symbol, name);
            targets.insert(instrument.key(), instrument);
        }

        if let Some((exchange, symbol)) = self.analytics_benchmark()? {
            let instrument =
                MarketInstrument::normalized(exchange, symbol, Some("Analytics benchmark".into()));
            targets.insert(instrument.key(), instrument);
        }

        let mut overrides = BTreeMap::new();
        let mut override_statement = self.connection.prepare(
            "SELECT i.exchange, i.symbol, ips.provider_symbol FROM instrument_provider_symbols ips \
             JOIN instruments i ON i.id = ips.instrument_id WHERE ips.provider_id = ?1",
        )?;
        let override_rows = override_statement.query_map(params![provider_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for row in override_rows {
            let (exchange, symbol, provider_symbol) = row?;
            overrides.insert(
                format!(
                    "{}:{}",
                    exchange.to_ascii_uppercase(),
                    symbol.to_ascii_uppercase()
                ),
                provider_symbol,
            );
        }

        Ok(targets
            .into_iter()
            .map(|(key, instrument)| instrument.with_provider_symbol(overrides.get(&key).cloned()))
            .collect())
    }

    pub fn set_provider_symbol(
        &self,
        exchange: &str,
        symbol: &str,
        provider_id: &str,
        provider_symbol: &str,
    ) -> Result<(), StorageError> {
        let instrument_id: i64 = self.connection.query_row(
            "SELECT id FROM instruments WHERE exchange=?1 AND symbol=?2",
            params![
                exchange.trim().to_ascii_uppercase(),
                symbol.trim().to_ascii_uppercase()
            ],
            |row| row.get(0),
        )?;
        self.connection.execute(
            "INSERT INTO instrument_provider_symbols(instrument_id, provider_id, provider_symbol) VALUES(?1, ?2, ?3) \
             ON CONFLICT(instrument_id, provider_id) DO UPDATE SET provider_symbol=excluded.provider_symbol",
            params![instrument_id, provider_id.trim().to_ascii_lowercase(), provider_symbol.trim()],
        )?;
        Ok(())
    }

    pub fn upsert_quotes_atomic(
        &mut self,
        quotes: &[Quote],
        history_enabled: bool,
    ) -> Result<usize, StorageError> {
        let tx = self.connection.transaction()?;
        let mut persisted = 0usize;
        for quote in quotes {
            if !quote.price.is_finite() || quote.price < 0.0 {
                continue;
            }
            let instrument = InstrumentRef::normalized(&quote.exchange, &quote.symbol, None);
            let instrument_id = upsert_instrument(&tx, &instrument)?;
            let observed_at = unix_ms_to_rfc3339(quote.observed_unix_ms)?;
            let freshness = freshness_to_db(quote.freshness);
            tx.execute(
                "INSERT INTO quotes(instrument_id, price, change_pct, observed_at, source, freshness) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT(instrument_id) DO UPDATE SET price=excluded.price, change_pct=excluded.change_pct, \
                 observed_at=excluded.observed_at, source=excluded.source, freshness=excluded.freshness",
                params![instrument_id, quote.price, quote.change_pct, &observed_at, &quote.source, freshness],
            )?;
            if history_enabled {
                tx.execute(
                    "INSERT OR IGNORE INTO quote_history(instrument_id, price, observed_at, source) VALUES (?1, ?2, ?3, ?4)",
                    params![instrument_id, quote.price, &observed_at, &quote.source],
                )?;
            }
            persisted += 1;
        }
        tx.commit()?;
        Ok(persisted)
    }

    pub fn latest_quotes(&self) -> Result<Vec<Quote>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT i.symbol, i.exchange, q.price, q.change_pct, q.observed_at, q.source, q.freshness \
             FROM quotes q JOIN instruments i ON i.id = q.instrument_id ORDER BY i.exchange, i.symbol",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut quotes = Vec::new();
        for row in rows {
            let (symbol, exchange, price, change_pct, observed_at, source, freshness) = row?;
            quotes.push(Quote {
                symbol,
                exchange,
                price,
                change_pct,
                observed_unix_ms: rfc3339_to_unix_ms(&observed_at)?,
                source,
                freshness: freshness_from_db(&freshness)?,
            });
        }
        Ok(quotes)
    }

    pub fn record_provider_success(
        &self,
        provider_id: &str,
        now_rfc3339: &str,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO market_provider_state(provider_id, consecutive_failures, last_success_at, last_error, next_retry_at) \
             VALUES (?1, 0, ?2, NULL, NULL) \
             ON CONFLICT(provider_id) DO UPDATE SET consecutive_failures=0, last_success_at=excluded.last_success_at, \
             last_error=NULL, next_retry_at=NULL",
            params![provider_id, now_rfc3339],
        )?;
        Ok(())
    }

    pub fn record_provider_failure(
        &self,
        provider_id: &str,
        error: &str,
        next_retry_at: Option<&str>,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO market_provider_state(provider_id, consecutive_failures, last_error, next_retry_at) \
             VALUES (?1, 1, ?2, ?3) \
             ON CONFLICT(provider_id) DO UPDATE SET consecutive_failures=market_provider_state.consecutive_failures+1, \
             last_error=excluded.last_error, next_retry_at=excluded.next_retry_at",
            params![provider_id, error, next_retry_at],
        )?;
        Ok(())
    }

    pub fn provider_state(
        &self,
        provider_id: &str,
    ) -> Result<Option<ProviderStateRecord>, StorageError> {
        self.connection.query_row(
            "SELECT provider_id, consecutive_failures, last_success_at, last_error, next_retry_at \
             FROM market_provider_state WHERE provider_id=?1",
            params![provider_id],
            |row| Ok(ProviderStateRecord {
                provider_id: row.get(0)?,
                consecutive_failures: row.get::<_, i64>(1)?.max(0) as u32,
                last_success_at: row.get(2)?,
                last_error: row.get(3)?,
                next_retry_at: row.get(4)?,
            }),
        ).optional().map_err(StorageError::from)
    }

    pub fn load_app_settings(&self) -> Result<AppSettings, StorageError> {
        let json: Option<String> = self
            .connection
            .query_row(
                "SELECT value_json FROM settings WHERE key='app_settings'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        match json {
            Some(value) => serde_json::from_str::<AppSettings>(&value)
                .map(|v| v.normalized())
                .map_err(|e| StorageError::InvalidJson(e.to_string())),
            None => Ok(AppSettings::default()),
        }
    }

    pub fn save_app_settings(&self, settings: &AppSettings) -> Result<(), StorageError> {
        let json = serde_json::to_string(&settings.clone().normalized())
            .map_err(|e| StorageError::InvalidJson(e.to_string()))?;
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default();
        self.connection.execute(
            "INSERT INTO settings(key,value_json,updated_at) VALUES('app_settings',?1,?2) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
            params![json, now],
        )?;
        Ok(())
    }

    pub fn upsert_alert_rule(&self, rule: &AlertRule) -> Result<(), StorageError> {
        let instrument_id = match (&rule.exchange, &rule.symbol) {
            (Some(exchange), Some(symbol)) => {
                let exchange = exchange.trim().to_ascii_uppercase();
                let symbol = symbol.trim().to_ascii_uppercase();
                self.connection.execute(
                    "INSERT INTO instruments(exchange, symbol, name) VALUES(?1, ?2, NULL) \
                     ON CONFLICT(exchange, symbol) DO NOTHING",
                    params![&exchange, &symbol],
                )?;
                Some(self.connection.query_row(
                    "SELECT id FROM instruments WHERE exchange=?1 AND symbol=?2",
                    params![&exchange, &symbol],
                    |row| row.get::<_, i64>(0),
                )?)
            }
            _ => None,
        };
        let condition_json = serde_json::to_string(&rule.condition)
            .map_err(|e| StorageError::InvalidJson(e.to_string()))?;
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default();
        self.connection.execute(
            "INSERT INTO alerts(id,instrument_id,enabled,condition_json,cooldown_seconds,reset_margin,last_triggered_at) VALUES(?1,?2,?3,?4,?5,?6,?7) \
             ON CONFLICT(id) DO UPDATE SET instrument_id=excluded.instrument_id, enabled=excluded.enabled, condition_json=excluded.condition_json, cooldown_seconds=excluded.cooldown_seconds, reset_margin=excluded.reset_margin, last_triggered_at=excluded.last_triggered_at",
            params![rule.id, instrument_id, rule.enabled as i64, condition_json, rule.cooldown_seconds as i64, rule.reset_margin, rule.last_triggered_at],
        )?;
        self.connection.execute(
            "INSERT INTO alert_rule_meta(alert_id,exchange,repeat_mode,market_hours_only,armed,last_value,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) \
             ON CONFLICT(alert_id) DO UPDATE SET exchange=excluded.exchange, repeat_mode=excluded.repeat_mode, market_hours_only=excluded.market_hours_only, armed=excluded.armed, last_value=excluded.last_value, updated_at=excluded.updated_at",
            params![rule.id, rule.exchange, repeat_mode_to_db(rule.repeat_mode), rule.market_hours_only as i64, rule.armed as i64, rule.last_value, rule.created_at.as_deref().unwrap_or(&now), now],
        )?;
        Ok(())
    }

    pub fn list_alert_rules(&self) -> Result<Vec<AlertRule>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT a.id,i.symbol,a.enabled,a.condition_json,a.cooldown_seconds,a.reset_margin,a.last_triggered_at,m.exchange,m.repeat_mode,m.market_hours_only,m.armed,m.last_value,m.created_at \
             FROM alerts a LEFT JOIN instruments i ON i.id=a.instrument_id LEFT JOIN alert_rule_meta m ON m.alert_id=a.id ORDER BY a.id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<f64>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, Option<i64>>(10)?,
                row.get::<_, Option<f64>>(11)?,
                row.get::<_, Option<String>>(12)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (
                id,
                symbol,
                enabled,
                condition_json,
                cooldown,
                reset_margin,
                last_triggered_at,
                exchange,
                repeat_mode,
                market_hours_only,
                armed,
                last_value,
                created_at,
            ) = row?;
            out.push(AlertRule {
                id,
                symbol,
                exchange,
                enabled: enabled != 0,
                condition: serde_json::from_str(&condition_json)
                    .map_err(|e| StorageError::InvalidJson(e.to_string()))?,
                cooldown_seconds: cooldown.max(0) as u64,
                reset_margin,
                repeat_mode: repeat_mode_from_db(repeat_mode.as_deref().unwrap_or("crossing"))?,
                market_hours_only: market_hours_only.unwrap_or(0) != 0,
                armed: armed.unwrap_or(1) != 0,
                last_value,
                last_triggered_at,
                created_at,
            });
        }
        Ok(out)
    }

    pub fn delete_alert_rule(&self, id: &str) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .execute("DELETE FROM alerts WHERE id=?1", params![id])?
            > 0)
    }

    pub fn update_alert_runtime(&self, rule: &AlertRule) -> Result<(), StorageError> {
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default();
        self.connection.execute(
            "UPDATE alerts SET enabled=?2, last_triggered_at=?3 WHERE id=?1",
            params![rule.id, rule.enabled as i64, rule.last_triggered_at],
        )?;
        self.connection.execute(
            "UPDATE alert_rule_meta SET armed=?2,last_value=?3,updated_at=?4 WHERE alert_id=?1",
            params![rule.id, rule.armed as i64, rule.last_value, now],
        )?;
        Ok(())
    }

    pub fn record_alert_event(&self, event: &AlertEvent) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO alert_events(alert_id,fired_at,metric_value,title,body,delivery_status,delivery_error) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![event.alert_id,event.fired_at,event.metric_value,event.title,event.body,event.delivery_status,event.delivery_error],
        )?;
        Ok(())
    }

    pub fn recent_alert_events(&self, limit: usize) -> Result<Vec<AlertEventRecord>, StorageError> {
        let mut stmt=self.connection.prepare("SELECT id,alert_id,fired_at,metric_value,title,body,delivery_status,delivery_error FROM alert_events ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit.clamp(1, 500) as i64], |row| {
            Ok(AlertEventRecord {
                id: row.get(0)?,
                alert_id: row.get(1)?,
                fired_at: row.get(2)?,
                metric_value: row.get(3)?,
                title: row.get(4)?,
                body: row.get(5)?,
                delivery_status: row.get(6)?,
                delivery_error: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn prune_alert_history(&self, cutoff_rfc3339: &str) -> Result<usize, StorageError> {
        Ok(self.connection.execute(
            "DELETE FROM alert_events WHERE fired_at < ?1",
            params![cutoff_rfc3339],
        )?)
    }

    pub fn insert_cash_flow(&self, flow: &CashFlow) -> Result<i64, StorageError> {
        if flow.amount <= Decimal::ZERO {
            return Err(StorageError::InvalidDecimal {
                field: "cash_flow.amount",
                value: flow.amount.to_string(),
            });
        }
        OffsetDateTime::parse(flow.occurred_at.trim(), &Rfc3339)
            .map_err(|_| StorageError::InvalidTimestamp(flow.occurred_at.clone()))?;
        let amount = decimal_to_db(flow.amount, "cash_flow.amount")?;
        let kind = cash_flow_kind_to_db(flow.kind);
        self.connection.execute(
            "INSERT INTO cash_flows(kind,amount,occurred_at,note,external_ref) VALUES(?1,?2,?3,?4,?5) 
             ON CONFLICT(external_ref) DO NOTHING",
            params![kind, amount, flow.occurred_at, flow.note.as_deref(), flow.external_ref.as_deref()],
        )?;
        if let Some(reference) = flow.external_ref.as_deref() {
            return Ok(self.connection.query_row(
                "SELECT id FROM cash_flows WHERE external_ref=?1",
                params![reference],
                |row| row.get(0),
            )?);
        }
        Ok(self.connection.last_insert_rowid())
    }

    pub fn list_cash_flows(&self) -> Result<Vec<CashFlow>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT id,kind,amount,occurred_at,note,external_ref FROM cash_flows ORDER BY occurred_at,id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, kind, amount, occurred_at, note, external_ref) = row?;
            out.push(CashFlow {
                id: Some(id),
                kind: cash_flow_kind_from_db(&kind)?,
                amount: decimal_from_db("cash_flow.amount", amount)?,
                occurred_at,
                note,
                external_ref,
            });
        }
        Ok(out)
    }

    pub fn delete_cash_flow(&self, id: i64) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .execute("DELETE FROM cash_flows WHERE id=?1", params![id])?
            > 0)
    }

    pub fn analytics_benchmark(&self) -> Result<Option<(String, String)>, StorageError> {
        let row: Option<(Option<String>, Option<String>)> = self
            .connection
            .query_row(
                "SELECT benchmark_exchange,benchmark_symbol FROM analytics_preferences WHERE id=1",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()?;
        Ok(row.and_then(|value| value.flatten_pair()))
    }

    pub fn set_analytics_benchmark(
        &self,
        exchange: Option<&str>,
        symbol: Option<&str>,
    ) -> Result<(), StorageError> {
        let exchange = exchange
            .map(|value| value.trim().to_ascii_uppercase())
            .filter(|value| !value.is_empty());
        let symbol = symbol
            .map(|value| value.trim().to_ascii_uppercase())
            .filter(|value| !value.is_empty());
        let (exchange, symbol) = match (exchange, symbol) {
            (Some(exchange), Some(symbol)) => (Some(exchange), Some(symbol)),
            _ => (None, None),
        };
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default();
        self.connection.execute(
            "INSERT INTO analytics_preferences(id,benchmark_exchange,benchmark_symbol,updated_at) VALUES(1,?1,?2,?3) 
             ON CONFLICT(id) DO UPDATE SET benchmark_exchange=excluded.benchmark_exchange, benchmark_symbol=excluded.benchmark_symbol, updated_at=excluded.updated_at",
            params![exchange,symbol,now],
        )?;
        Ok(())
    }

    pub fn upsert_performance_snapshot(
        &mut self,
        observed_unix_ms: i64,
        portfolio_value: f64,
        holding_values: &[(String, String, f64, f64)],
        source: &str,
    ) -> Result<bool, StorageError> {
        if !portfolio_value.is_finite() || portfolio_value < 0.0 {
            return Ok(false);
        }
        let portfolio = self.derive_portfolio()?;
        let cash_flows = self.list_cash_flows()?;
        let investment_income: f64 = cash_flows
            .iter()
            .filter(|flow| flow.kind.is_income())
            .map(|flow| flow.amount.to_f64().unwrap_or(0.0))
            .sum();
        let net_external_flow: f64 = cash_flows
            .iter()
            .map(|flow| {
                flow.kind
                    .signed_external_amount(flow.amount)
                    .to_f64()
                    .unwrap_or(0.0)
            })
            .sum();
        let bucket_unix_ms = observed_unix_ms.div_euclid(300_000).saturating_mul(300_000);
        let observed_at = unix_ms_to_rfc3339(bucket_unix_ms)?;
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO performance_snapshots(bucket_unix_ms,observed_at,portfolio_value,open_cost_basis,realized_gross_pnl,fees_paid,investment_income,net_external_flow,source) 
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) 
             ON CONFLICT(bucket_unix_ms) DO UPDATE SET observed_at=excluded.observed_at,portfolio_value=excluded.portfolio_value,open_cost_basis=excluded.open_cost_basis,realized_gross_pnl=excluded.realized_gross_pnl,fees_paid=excluded.fees_paid,investment_income=excluded.investment_income,net_external_flow=excluded.net_external_flow,source=excluded.source",
            params![bucket_unix_ms,observed_at,portfolio_value,decimal_to_db(portfolio.total_open_cost_basis(),"open_cost_basis")?,decimal_to_db(portfolio.realized_gross_pnl,"realized_gross_pnl")?,decimal_to_db(portfolio.fees_paid,"fees_paid")?,investment_income,net_external_flow,source],
        )?;
        let snapshot_id: i64 = tx.query_row(
            "SELECT id FROM performance_snapshots WHERE bucket_unix_ms=?1",
            params![bucket_unix_ms],
            |row| row.get(0),
        )?;
        tx.execute(
            "DELETE FROM performance_holding_snapshots WHERE snapshot_id=?1",
            params![snapshot_id],
        )?;
        for (exchange, symbol, price, current_value) in holding_values {
            let instrument_id: Option<i64> = tx
                .query_row(
                    "SELECT id FROM instruments WHERE exchange=?1 AND symbol=?2",
                    params![exchange.to_ascii_uppercase(), symbol.to_ascii_uppercase()],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(instrument_id) = instrument_id else {
                continue;
            };
            let holding = portfolio.open_holdings().find(|holding| {
                holding.instrument.exchange.eq_ignore_ascii_case(exchange)
                    && holding.instrument.symbol.eq_ignore_ascii_case(symbol)
            });
            let Some(holding) = holding else { continue };
            tx.execute(
                "INSERT INTO performance_holding_snapshots(snapshot_id,instrument_id,quantity,price,current_value,open_cost_basis) VALUES(?1,?2,?3,?4,?5,?6)",
                params![snapshot_id,instrument_id,decimal_to_db(holding.quantity,"quantity")?,price,current_value,decimal_to_db(holding.gross_cost_basis(),"open_cost_basis")?],
            )?;
        }
        tx.commit()?;
        Ok(true)
    }

    pub fn performance_snapshots(
        &self,
        limit: usize,
    ) -> Result<Vec<PerformanceSnapshot>, StorageError> {
        let mut stmt=self.connection.prepare(
            "SELECT observed_at,portfolio_value,open_cost_basis,realized_gross_pnl,fees_paid,investment_income,net_external_flow,source 
             FROM performance_snapshots ORDER BY bucket_unix_ms DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit.clamp(2, 50_000) as i64], |row| {
            Ok(PerformanceSnapshot {
                observed_at: row.get(0)?,
                portfolio_value: row.get(1)?,
                open_cost_basis: row.get(2)?,
                realized_gross_pnl: row.get(3)?,
                fees_paid: row.get(4)?,
                investment_income: row.get(5)?,
                net_external_flow: row.get(6)?,
                source: row.get(7)?,
            })
        })?;
        let mut out = rows.collect::<Result<Vec<_>, _>>()?;
        out.reverse();
        Ok(out)
    }

    pub fn quote_history_points(
        &self,
        exchange: &str,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<(String, f64)>, StorageError> {
        let instrument_id: Option<i64> = self
            .connection
            .query_row(
                "SELECT id FROM instruments WHERE exchange=?1 AND symbol=?2",
                params![
                    exchange.trim().to_ascii_uppercase(),
                    symbol.trim().to_ascii_uppercase()
                ],
                |row| row.get(0),
            )
            .optional()?;
        let Some(instrument_id) = instrument_id else {
            return Ok(Vec::new());
        };
        let mut stmt=self.connection.prepare(
            "SELECT observed_at,price FROM quote_history WHERE instrument_id=?1 ORDER BY observed_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![instrument_id, limit.clamp(2, 5000) as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })?;
        let mut out = rows.collect::<Result<Vec<_>, _>>()?;
        out.reverse();
        Ok(out)
    }

    pub fn benchmark_price_range(
        &self,
        exchange: &str,
        symbol: &str,
        start: &str,
        end: &str,
    ) -> Result<Option<(f64, f64)>, StorageError> {
        let instrument_id: Option<i64> = self
            .connection
            .query_row(
                "SELECT id FROM instruments WHERE exchange=?1 AND symbol=?2",
                params![
                    exchange.trim().to_ascii_uppercase(),
                    symbol.trim().to_ascii_uppercase()
                ],
                |row| row.get(0),
            )
            .optional()?;
        let Some(instrument_id) = instrument_id else {
            return Ok(None);
        };
        let first:Option<f64>=self.connection.query_row(
            "SELECT price FROM quote_history WHERE instrument_id=?1 AND observed_at>=?2 AND observed_at<=?3 ORDER BY observed_at ASC LIMIT 1",
            params![instrument_id,start,end],|row|row.get(0),
        ).optional()?;
        let last:Option<f64>=self.connection.query_row(
            "SELECT price FROM quote_history WHERE instrument_id=?1 AND observed_at>=?2 AND observed_at<=?3 ORDER BY observed_at DESC LIMIT 1",
            params![instrument_id,start,end],|row|row.get(0),
        ).optional()?;
        Ok(match (first, last) {
            (Some(first), Some(last)) if first > 0.0 => Some((first, last)),
            _ => None,
        })
    }

    pub fn save_reliability_state(&self, key: &str, value_json: &str) -> Result<(), StorageError> {
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default();
        self.connection.execute(
            "INSERT INTO reliability_state(key,value_json,updated_at) VALUES(?1,?2,?3) \
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
            params![key, value_json, now],
        )?;
        Ok(())
    }

    pub fn reliability_state(&self, key: &str) -> Result<Option<(String, String)>, StorageError> {
        self.connection
            .query_row(
                "SELECT value_json,updated_at FROM reliability_state WHERE key=?1",
                params![key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn database_health(&self) -> Result<crate::reliability::DatabaseHealth, StorageError> {
        let quick_check: String = self
            .connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        let database_bytes: u64 = self
            .connection
            .path()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .unwrap_or(0);
        Ok(crate::reliability::DatabaseHealth {
            ok: quick_check.eq_ignore_ascii_case("ok"),
            quick_check,
            schema_version: self.schema_version()?,
            database_bytes,
        })
    }

    pub fn prune_history(
        &mut self,
        quote_cutoff: &str,
        performance_cutoff: &str,
        alert_cutoff: &str,
    ) -> Result<HistoryPruneReport, StorageError> {
        let tx = self.connection.transaction()?;
        let quote_history = tx.execute(
            "DELETE FROM quote_history WHERE observed_at < ?1",
            params![quote_cutoff],
        )?;
        let performance_snapshots = tx.execute(
            "DELETE FROM performance_snapshots WHERE observed_at < ?1",
            params![performance_cutoff],
        )?;
        let alert_events = tx.execute(
            "DELETE FROM alert_events WHERE fired_at < ?1",
            params![alert_cutoff],
        )?;
        tx.commit()?;
        Ok(HistoryPruneReport {
            quote_history,
            performance_snapshots,
            alert_events,
        })
    }

    pub fn checkpoint_wal(&self) -> Result<(), StorageError> {
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }
}

fn upsert_instrument(
    transaction: &SqlTransaction<'_>,
    instrument: &InstrumentRef,
) -> Result<i64, rusqlite::Error> {
    transaction.execute(
        "INSERT INTO instruments(exchange, symbol, name) VALUES (?1, ?2, ?3) \
         ON CONFLICT(exchange, symbol) DO UPDATE SET name = COALESCE(excluded.name, instruments.name)",
        params![&instrument.exchange, &instrument.symbol, instrument.name.as_deref()],
    )?;
    transaction.query_row(
        "SELECT id FROM instruments WHERE exchange = ?1 AND symbol = ?2",
        params![&instrument.exchange, &instrument.symbol],
        |row| row.get(0),
    )
}

fn insert_trade(
    transaction: &SqlTransaction<'_>,
    instrument_id: i64,
    trade: &Trade,
) -> Result<bool, StorageError> {
    let quantity = decimal_to_db(trade.quantity, "quantity")?;
    let price = decimal_to_db(trade.price, "price")?;
    let fees = decimal_to_db(trade.fees, "fees")?;
    let side = match trade.side {
        Side::Buy => "buy",
        Side::Sell => "sell",
    };
    let changed = transaction.execute(
        "INSERT INTO transactions(instrument_id, side, quantity, price, fees, executed_at, external_ref) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(external_ref) DO NOTHING",
        params![instrument_id, side, quantity, price, fees, &trade.executed_at, trade.external_ref.as_deref()],
    )?;
    Ok(changed == 1)
}

fn load_trades(connection: &Connection) -> Result<Vec<Trade>, StorageError> {
    let mut statement = connection.prepare(
        "SELECT t.id, i.exchange, i.symbol, i.name, t.side, t.quantity, t.price, t.fees, t.executed_at, t.external_ref \
         FROM transactions t JOIN instruments i ON i.id = t.instrument_id ORDER BY t.executed_at, t.id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, f64>(5)?,
            row.get::<_, f64>(6)?,
            row.get::<_, f64>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, Option<String>>(9)?,
        ))
    })?;
    let mut trades = Vec::new();
    for row in rows {
        let (id, exchange, symbol, name, side, quantity, price, fees, executed_at, external_ref) =
            row?;
        trades.push(Trade {
            id: Some(id),
            instrument: InstrumentRef::normalized(exchange, symbol, name),
            side: if side.eq_ignore_ascii_case("buy") {
                Side::Buy
            } else {
                Side::Sell
            },
            quantity: decimal_from_db("quantity", quantity)?,
            price: decimal_from_db("price", price)?,
            fees: decimal_from_db("fees", fees)?,
            executed_at,
            external_ref,
        });
    }
    Ok(trades)
}

fn decimal_to_db(value: Decimal, field: &'static str) -> Result<f64, StorageError> {
    value.to_f64().ok_or_else(|| StorageError::InvalidDecimal {
        field,
        value: value.to_string(),
    })
}
fn decimal_from_db(field: &'static str, value: f64) -> Result<Decimal, StorageError> {
    Decimal::from_str(&format!("{value:.12}"))
        .map(|v| v.normalize())
        .map_err(|_| StorageError::InvalidDecimal {
            field,
            value: value.to_string(),
        })
}
fn cash_flow_kind_to_db(value: CashFlowKind) -> &'static str {
    match value {
        CashFlowKind::Deposit => "deposit",
        CashFlowKind::Withdrawal => "withdrawal",
        CashFlowKind::Dividend => "dividend",
        CashFlowKind::Interest => "interest",
    }
}
fn cash_flow_kind_from_db(value: &str) -> Result<CashFlowKind, StorageError> {
    match value {
        "deposit" => Ok(CashFlowKind::Deposit),
        "withdrawal" => Ok(CashFlowKind::Withdrawal),
        "dividend" => Ok(CashFlowKind::Dividend),
        "interest" => Ok(CashFlowKind::Interest),
        other => Err(StorageError::InvalidJson(format!(
            "invalid cash flow kind: {other}"
        ))),
    }
}

trait FlattenPair {
    fn flatten_pair(self) -> Option<(String, String)>;
}
impl FlattenPair for (Option<String>, Option<String>) {
    fn flatten_pair(self) -> Option<(String, String)> {
        match self {
            (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() => Some((a, b)),
            _ => None,
        }
    }
}

fn repeat_mode_to_db(value: AlertRepeatMode) -> &'static str {
    match value {
        AlertRepeatMode::Once => "once",
        AlertRepeatMode::Crossing => "crossing",
        AlertRepeatMode::RepeatWhileTrue => "repeat_while_true",
    }
}
fn repeat_mode_from_db(value: &str) -> Result<AlertRepeatMode, StorageError> {
    match value {
        "once" => Ok(AlertRepeatMode::Once),
        "crossing" => Ok(AlertRepeatMode::Crossing),
        "repeat_while_true" => Ok(AlertRepeatMode::RepeatWhileTrue),
        other => Err(StorageError::InvalidJson(format!(
            "invalid alert repeat mode: {other}"
        ))),
    }
}
fn freshness_to_db(f: QuoteFreshness) -> &'static str {
    match f {
        QuoteFreshness::Live => "live",
        QuoteFreshness::Fresh => "fresh",
        QuoteFreshness::Delayed => "delayed",
        QuoteFreshness::Stale => "stale",
        QuoteFreshness::Offline => "offline",
    }
}
fn freshness_from_db(value: &str) -> Result<QuoteFreshness, StorageError> {
    match value {
        "live" => Ok(QuoteFreshness::Live),
        "fresh" => Ok(QuoteFreshness::Fresh),
        "delayed" => Ok(QuoteFreshness::Delayed),
        "stale" => Ok(QuoteFreshness::Stale),
        "offline" => Ok(QuoteFreshness::Offline),
        other => Err(StorageError::InvalidFreshness(other.into())),
    }
}
fn unix_ms_to_rfc3339(value: i64) -> Result<String, StorageError> {
    let nanos = i128::from(value).saturating_mul(1_000_000);
    OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .map_err(|_| StorageError::InvalidTimestamp(value.to_string()))?
        .format(&Rfc3339)
        .map_err(|_| StorageError::InvalidTimestamp(value.to_string()))
}
fn rfc3339_to_unix_ms(value: &str) -> Result<i64, StorageError> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| StorageError::InvalidTimestamp(value.into()))?;
    Ok(parsed.unix_timestamp_nanos().saturating_div(1_000_000) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(value: &str) -> Decimal {
        Decimal::from_str(value).unwrap()
    }
    fn trade(external_ref: &str, side: Side, quantity: &str, price: &str) -> Trade {
        Trade {
            id: None,
            instrument: InstrumentRef::normalized("NSE", "DEMOTECH", Some("Demo Technology".into())),
            side,
            quantity: d(quantity),
            price: d(price),
            fees: Decimal::ZERO,
            executed_at: "2026-11-08T10:00:00Z".into(),
            external_ref: Some(external_ref.into()),
        }
    }

    #[test]
    fn migrations_reach_current_schema() {
        let repo = PortfolioRepository::in_memory().unwrap();
        assert_eq!(
            repo.schema_version().unwrap(),
            PortfolioRepository::expected_schema_version()
        );
    }
    #[test]
    fn duplicate_external_refs_are_idempotent() {
        let mut repo = PortfolioRepository::in_memory().unwrap();
        let row = trade("same-id", Side::Buy, "2", "257.52");
        let first = repo
            .insert_trades_atomic(std::slice::from_ref(&row))
            .unwrap();
        let second = repo.insert_trades_atomic(&[row]).unwrap();
        assert_eq!(first.inserted, 1);
        assert_eq!(second.inserted, 0);
        assert_eq!(second.skipped_duplicates, 1);
        assert_eq!(repo.transaction_count().unwrap(), 1);
    }
    #[test]
    fn stored_trades_derive_portfolio() {
        let mut repo = PortfolioRepository::in_memory().unwrap();
        repo.insert_trades_atomic(&[
            trade("b1", Side::Buy, "2", "100"),
            trade("s1", Side::Sell, "1", "120"),
        ])
        .unwrap();
        let portfolio = repo.derive_portfolio().unwrap();
        assert_eq!(portfolio.positions[0].quantity, d("1"));
        assert_eq!(portfolio.realized_gross_pnl, d("20"));
    }
    #[test]
    fn oversell_batch_rolls_back_atomically() {
        let mut repo = PortfolioRepository::in_memory().unwrap();
        repo.insert_trades_atomic(&[trade("b1", Side::Buy, "1", "100")])
            .unwrap();
        let before = repo.transaction_count().unwrap();
        let error = repo
            .insert_trades_atomic(&[trade("s1", Side::Sell, "2", "120")])
            .unwrap_err();
        assert!(matches!(
            error,
            StorageError::Portfolio(PortfolioError::Oversell { .. })
        ));
        assert_eq!(repo.transaction_count().unwrap(), before);
    }
    #[test]
    fn quote_cache_round_trip_and_provider_health() {
        let mut repo = PortfolioRepository::in_memory().unwrap();
        let quote = Quote {
            symbol: "DEMOTECH".into(),
            exchange: "NSE".into(),
            price: 299.3,
            change_pct: Some(1.85),
            observed_unix_ms: 1_799_999_000_000,
            source: "test".into(),
            freshness: QuoteFreshness::Delayed,
        };
        assert_eq!(
            repo.upsert_quotes_atomic(std::slice::from_ref(&quote), true)
                .unwrap(),
            1
        );
        assert_eq!(repo.latest_quotes().unwrap(), vec![quote]);
        repo.record_provider_failure("test", "down", None).unwrap();
        assert_eq!(
            repo.provider_state("test")
                .unwrap()
                .unwrap()
                .consecutive_failures,
            1
        );
        repo.record_provider_success("test", "2026-11-08T10:00:00Z")
            .unwrap();
        let state = repo.provider_state("test").unwrap().unwrap();
        assert_eq!(state.consecutive_failures, 0);
        assert!(state.last_error.is_none());
    }
}
