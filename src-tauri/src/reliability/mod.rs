use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    settings::{ReliabilitySettings, SettingsState},
    storage::{PortfolioRepository, SCHEMA_VERSION},
};

const DB_NAME: &str = "stockwatch.db";
const PENDING_RESTORE_NAME: &str = "stockwatch.restore.pending.db";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseHealth {
    pub ok: bool,
    pub quick_check: String,
    pub schema_version: i64,
    pub database_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub file_name: String,
    pub bytes: u64,
    pub modified_unix_ms: i64,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceReport {
    pub integrity_ok: bool,
    pub backup_created: Option<String>,
    pub backups_removed: usize,
    pub quote_history_removed: usize,
    pub performance_snapshots_removed: usize,
    pub alert_events_removed: usize,
    pub wal_checkpointed: bool,
}

#[derive(Clone)]
pub struct EventLogger {
    path: PathBuf,
    max_bytes: u64,
    keep_files: usize,
    lock: Arc<Mutex<()>>,
}

impl EventLogger {
    pub fn new(log_dir: impl AsRef<Path>, settings: &ReliabilitySettings) -> std::io::Result<Self> {
        fs::create_dir_all(log_dir.as_ref())?;
        Ok(Self {
            path: log_dir.as_ref().join("stockwatch.log"),
            max_bytes: settings.event_log_max_bytes,
            keep_files: settings.event_log_keep_files,
            lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn append(&self, level: &str, event: &str, detail: &str) {
        let _guard = self.lock.lock().ok();
        let _ = self.rotate_if_needed();
        let timestamp = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default();
        let detail = detail.replace(['\r', '\n'], " ");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(
                file,
                "{timestamp}\t{}\t{}\t{}",
                level.trim(),
                event.trim(),
                detail
            );
            let _ = file.flush();
        }
    }

    fn rotate_if_needed(&self) -> std::io::Result<()> {
        let current = fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        if current < self.max_bytes.max(64 * 1024) {
            return Ok(());
        }
        for index in (1..=self.keep_files.max(1)).rev() {
            let source = if index == 1 {
                self.path.clone()
            } else {
                self.path.with_extension(format!("log.{}", index - 1))
            };
            let target = self.path.with_extension(format!("log.{index}"));
            if source.exists() {
                if index == self.keep_files.max(1) && target.exists() {
                    let _ = fs::remove_file(&target);
                }
                let _ = fs::rename(&source, &target);
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct MaintenanceService {
    app_data: PathBuf,
    db_path: PathBuf,
    settings: SettingsState,
    logger: EventLogger,
}

impl MaintenanceService {
    pub fn new(app_data: PathBuf, settings: SettingsState) -> Result<Self, String> {
        let logger = EventLogger::new(app_data.join("logs"), &settings.snapshot().reliability)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            db_path: app_data.join(DB_NAME),
            app_data,
            settings,
            logger,
        })
    }

    pub fn logger(&self) -> EventLogger {
        self.logger.clone()
    }

    pub fn run_once(&self) -> Result<MaintenanceReport, String> {
        let settings = self.settings.snapshot();
        let reliability = settings.reliability;
        let mut report = MaintenanceReport::default();
        let mut repo = PortfolioRepository::open(&self.db_path).map_err(|e| e.to_string())?;
        let health = repo.database_health().map_err(|e| e.to_string())?;
        report.integrity_ok = health.ok;
        if !health.ok {
            self.logger
                .append("ERROR", "integrity", &health.quick_check);
            return Err(format!(
                "database quick_check failed: {}",
                health.quick_check
            ));
        }

        if reliability.backup_enabled
            && backup_due(
                &self.app_data.join("backups"),
                reliability.backup_interval_hours,
            )
        {
            let backup = create_backup(&self.db_path, &self.app_data.join("backups"), "auto")?;
            report.backup_created = Some(backup.path.clone());
            self.logger
                .append("INFO", "backup_created", &backup.file_name);
        }
        report.backups_removed = prune_backups(
            &self.app_data.join("backups"),
            reliability.backup_keep_count,
        )?;

        let now = OffsetDateTime::now_utc();
        let quote_cutoff =
            now - time::Duration::days(i64::from(reliability.quote_history_retention_days));
        let performance_cutoff =
            now - time::Duration::days(i64::from(reliability.performance_retention_days));
        let alert_cutoff =
            now - time::Duration::days(i64::from(settings.notifications.retain_history_days));
        let pruned = repo
            .prune_history(
                &quote_cutoff.format(&Rfc3339).unwrap_or_default(),
                &performance_cutoff.format(&Rfc3339).unwrap_or_default(),
                &alert_cutoff.format(&Rfc3339).unwrap_or_default(),
            )
            .map_err(|e| e.to_string())?;
        report.quote_history_removed = pruned.quote_history;
        report.performance_snapshots_removed = pruned.performance_snapshots;
        report.alert_events_removed = pruned.alert_events;
        repo.checkpoint_wal().map_err(|e| e.to_string())?;
        report.wal_checkpointed = true;
        let report_json = serde_json::to_string(&report).unwrap_or_else(|_| "{}".into());
        let _ = repo.save_reliability_state("last_maintenance", &report_json);
        self.logger.append("INFO", "maintenance", &report_json);
        Ok(report)
    }
}

pub fn prepare_database(app_data: &Path) -> Result<Option<String>, String> {
    fs::create_dir_all(app_data).map_err(|e| e.to_string())?;
    let db_path = app_data.join(DB_NAME);
    apply_pending_restore(app_data)?;
    if !db_path.exists() {
        return Ok(None);
    }

    let health = inspect_database(&db_path)?;
    if health.ok {
        if health.schema_version > SCHEMA_VERSION {
            return Err(format!(
                "database schema {} is newer than supported schema {}",
                health.schema_version, SCHEMA_VERSION
            ));
        }
        if health.schema_version < SCHEMA_VERSION {
            let _ = create_backup(&db_path, &app_data.join("backups"), "pre_migration")?;
            return Ok(Some(format!(
                "pre-migration backup created for schema {}",
                health.schema_version
            )));
        }
        return Ok(None);
    }

    let backups = list_backups(&app_data.join("backups"))?;
    let replacement = backups.into_iter().find(|b| b.verified).ok_or_else(|| {
        format!(
            "database is corrupt ({}) and no verified backup is available",
            health.quick_check
        )
    })?;
    let corrupt_path = app_data.join(format!("stockwatch.corrupt.{}.db", timestamp_slug()));
    fs::rename(&db_path, &corrupt_path).map_err(|e| e.to_string())?;
    fs::copy(&replacement.path, &db_path).map_err(|e| e.to_string())?;
    remove_sqlite_sidecars(&db_path);
    let recovered = inspect_database(&db_path)?;
    if !recovered.ok {
        let _ = fs::remove_file(&db_path);
        let _ = fs::rename(&corrupt_path, &db_path);
        return Err(format!(
            "verified backup failed after recovery copy: {}",
            recovered.quick_check
        ));
    }
    Ok(Some(format!(
        "recovered from {}; corrupt database preserved at {}",
        replacement.file_name,
        corrupt_path.display()
    )))
}

pub fn create_backup(db_path: &Path, backup_dir: &Path, label: &str) -> Result<BackupInfo, String> {
    if !db_path.exists() {
        return Err("database does not exist".into());
    }
    fs::create_dir_all(backup_dir).map_err(|e| e.to_string())?;
    let source_health = inspect_database(db_path)?;
    if !source_health.ok {
        return Err(format!(
            "refusing to back up unhealthy database: {}",
            source_health.quick_check
        ));
    }
    let file_name = format!(
        "stockwatch.{}.{}.db",
        sanitize_label(label),
        timestamp_slug()
    );
    let destination = backup_dir.join(&file_name);
    let connection = Connection::open(db_path).map_err(|e| e.to_string())?;
    let _ = connection.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
    let escaped = destination.to_string_lossy().replace('\'', "''");
    connection
        .execute_batch(&format!("VACUUM INTO '{escaped}';"))
        .map_err(|e| e.to_string())?;
    sync_file(&destination)?;
    let health = inspect_database(&destination)?;
    if !health.ok {
        let _ = fs::remove_file(&destination);
        return Err(format!(
            "backup verification failed: {}",
            health.quick_check
        ));
    }
    backup_info(&destination, true)
}

pub fn stage_restore(app_data: &Path, backup_path: &Path) -> Result<DatabaseHealth, String> {
    let health = inspect_database(backup_path)?;
    if !health.ok {
        return Err(format!(
            "restore source failed quick_check: {}",
            health.quick_check
        ));
    }
    if health.schema_version > SCHEMA_VERSION {
        return Err(format!(
            "backup schema {} is newer than supported schema {}",
            health.schema_version, SCHEMA_VERSION
        ));
    }
    let pending = app_data.join(PENDING_RESTORE_NAME);
    let temporary = app_data.join(format!("{PENDING_RESTORE_NAME}.tmp"));
    let _ = fs::remove_file(&temporary);
    fs::copy(backup_path, &temporary).map_err(|e| e.to_string())?;
    sync_file(&temporary)?;
    let copied = inspect_database(&temporary)?;
    if !copied.ok {
        let _ = fs::remove_file(&temporary);
        return Err("copied restore image failed verification".into());
    }
    if pending.exists() {
        fs::remove_file(&pending).map_err(|e| e.to_string())?;
    }
    fs::rename(&temporary, &pending).map_err(|e| e.to_string())?;
    Ok(copied)
}

pub fn apply_pending_restore(app_data: &Path) -> Result<bool, String> {
    let pending = app_data.join(PENDING_RESTORE_NAME);
    if !pending.exists() {
        return Ok(false);
    }
    let pending_health = inspect_database(&pending)?;
    if !pending_health.ok || pending_health.schema_version > SCHEMA_VERSION {
        return Err("pending restore image is invalid or from a newer schema".into());
    }
    let db_path = app_data.join(DB_NAME);
    let recovery = app_data.join(format!("stockwatch.pre_restore.{}.db", timestamp_slug()));
    if db_path.exists() {
        fs::rename(&db_path, &recovery).map_err(|e| e.to_string())?;
    }
    remove_sqlite_sidecars(&db_path);
    if let Err(error) = fs::rename(&pending, &db_path) {
        if recovery.exists() {
            let _ = fs::rename(&recovery, &db_path);
        }
        return Err(error.to_string());
    }
    let final_health = inspect_database(&db_path)?;
    if !final_health.ok {
        let _ = fs::remove_file(&db_path);
        if recovery.exists() {
            let _ = fs::rename(&recovery, &db_path);
        }
        return Err(format!(
            "restored database failed verification: {}",
            final_health.quick_check
        ));
    }
    Ok(true)
}

pub fn inspect_database(path: &Path) -> Result<DatabaseHealth, String> {
    if !path.exists() {
        return Err(format!("database not found: {}", path.display()));
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    let quick_check: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let schema_version: i64 = connection
        .query_row(
            "SELECT value FROM schema_meta WHERE key='schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let database_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    Ok(DatabaseHealth {
        ok: quick_check.eq_ignore_ascii_case("ok"),
        quick_check,
        schema_version,
        database_bytes,
    })
}

pub fn list_backups(backup_dir: &Path) -> Result<Vec<BackupInfo>, String> {
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }
    let mut rows = Vec::new();
    for entry in fs::read_dir(backup_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("db") {
            continue;
        }
        let verified = inspect_database(&path)
            .map(|h| h.ok && h.schema_version <= SCHEMA_VERSION)
            .unwrap_or(false);
        rows.push(backup_info(&path, verified)?);
    }
    rows.sort_by(|a, b| {
        b.modified_unix_ms
            .cmp(&a.modified_unix_ms)
            .then_with(|| b.file_name.cmp(&a.file_name))
    });
    Ok(rows)
}

pub fn prune_backups(backup_dir: &Path, keep_count: usize) -> Result<usize, String> {
    let backups = list_backups(backup_dir)?;
    let keep_count = keep_count.clamp(1, 100);
    let mut removed = 0;
    for backup in backups.into_iter().skip(keep_count) {
        fs::remove_file(&backup.path).map_err(|e| e.to_string())?;
        removed += 1;
    }
    Ok(removed)
}

fn backup_due(backup_dir: &Path, interval_hours: u16) -> bool {
    let newest = list_backups(backup_dir)
        .ok()
        .and_then(|v| v.into_iter().next());
    let Some(newest) = newest else { return true };
    let elapsed = OffsetDateTime::now_utc()
        .unix_timestamp_nanos()
        .saturating_div(1_000_000) as i64
        - newest.modified_unix_ms;
    elapsed >= i64::from(interval_hours.max(1)) * 3_600_000
}

fn backup_info(path: &Path, verified: bool) -> Result<BackupInfo, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let modified_unix_ms = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0);
    Ok(BackupInfo {
        path: path.to_string_lossy().to_string(),
        file_name: path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_string(),
        bytes: metadata.len(),
        modified_unix_ms,
        verified,
    })
}

fn sync_file(path: &Path) -> Result<(), String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())
}

fn remove_sqlite_sidecars(db_path: &Path) {
    for suffix in ["-wal", "-shm"] {
        let path = PathBuf::from(format!("{}{}", db_path.to_string_lossy(), suffix));
        let _ = fs::remove_file(path);
    }
}

fn timestamp_slug() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{}-{:02}-{:02}T{:02}{:02}{:02}.{:03}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.nanosecond() / 1_000_000
    )
}

fn sanitize_label(label: &str) -> String {
    let value: String = label
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if value.is_empty() {
        "backup".into()
    } else {
        value
    }
}

pub fn spawn_maintenance_worker(service: MaintenanceService) {
    tauri::async_runtime::spawn(async move {
        // Avoid doing disk maintenance during the startup hot path.
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        loop {
            let interval = service
                .settings
                .snapshot()
                .reliability
                .maintenance_interval_minutes
                .clamp(15, 1440);
            if let Err(error) = service.run_once() {
                service.logger.append("ERROR", "maintenance_failed", &error);
            }
            tokio::time::sleep(std::time::Duration::from_secs(u64::from(interval) * 60)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_path_safe() {
        assert_eq!(sanitize_label("pre migration/../"), "premigration");
        assert_eq!(sanitize_label(""), "backup");
    }
}
