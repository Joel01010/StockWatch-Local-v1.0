pub mod alerts;
pub mod analytics;
pub mod commands;
pub mod imports;
pub mod market;
pub mod notifications;
pub mod portfolio;
pub mod reliability;
pub mod runtime;
pub mod scheduler;
pub mod settings;
pub mod storage;
pub mod tray;

use std::sync::Arc;

use alerts::AlertService;
use market::{spawn_market_worker, MarketDataService};
use reliability::{prepare_database, spawn_maintenance_worker, MaintenanceService};
use runtime::RuntimeState;
use settings::SettingsState;
use storage::PortfolioRepository;
use tauri::{Manager, RunEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

pub fn run() {
    let runtime = RuntimeState::default();
    let setup_runtime = runtime.clone();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(runtime)
        .setup(move |app| {
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;
            let db_path = app_data.join("stockwatch.db");
            let recovery_note = prepare_database(&app_data).map_err(std::io::Error::other)?;
            let repo = PortfolioRepository::open(&db_path)?;
            let settings = repo.load_app_settings().unwrap_or_default();
            let settings_state = SettingsState::new(settings.clone());
            app.manage(settings_state.clone());
            let maintenance = MaintenanceService::new(app_data.clone(), settings_state.clone())
                .map_err(std::io::Error::other)?;
            if let Some(note) = recovery_note {
                maintenance
                    .logger()
                    .append("WARN", "startup_recovery", &note);
            }
            app.manage(maintenance.clone());

            // Keep the persisted startup preference aligned with the native OS registration.
            // Qualification launches use isolated AppData; they must never let the isolated
            // default setting overwrite the real user's Windows autostart state.
            let qualification_mode = std::env::var("STOCKWATCH_QUALIFICATION_MODE")
                .ok()
                .as_deref()
                == Some("1");
            if !qualification_mode {
                let autostart_result = if settings.launch_at_startup {
                    app.autolaunch().enable()
                } else {
                    app.autolaunch().disable()
                };
                if let Err(error) = autostart_result {
                    maintenance
                        .logger()
                        .append("WARN", "autostart_sync", &error.to_string());
                }
            }

            tray::install_native_tray(app, setup_runtime.clone(), settings_state.clone())?;

            let service = MarketDataService::new(
                db_path.clone(),
                setup_runtime.clone(),
                settings.market_data.clone(),
            )?
            .with_logger(maintenance.logger());
            let _ = service.refresh_runtime_from_cache();
            app.manage(service.clone());

            let alerts = AlertService::new(db_path, setup_runtime.clone(), settings_state.clone());
            let _ = alerts.refresh_active_count();
            app.manage(alerts.clone());

            // Qualification-only hooks used by the Phase 8 Windows scripts. They are inert in
            // normal launches and let the release gate exercise native paths without an agent.
            let mut qualification_report = Vec::new();
            if let Ok(action) = std::env::var("STOCKWATCH_QUALIFICATION_AUTOSTART") {
                let manager = app.autolaunch();
                let result = match action.trim().to_ascii_lowercase().as_str() {
                    "enable" => manager.enable(),
                    "disable" => manager.disable(),
                    _ => Ok(()),
                };
                qualification_report.push(format!(
                    "autostart_action={}:{}",
                    action,
                    if result.is_ok() { "ok" } else { "error" }
                ));
                match manager.is_enabled() {
                    Ok(value) => qualification_report.push(format!("autostart_enabled={value}")),
                    Err(error) => {
                        qualification_report.push(format!("autostart_status_error={error}"))
                    }
                }
            }
            if std::env::var("STOCKWATCH_QUALIFICATION_NOTIFY")
                .ok()
                .as_deref()
                == Some("1")
            {
                let result = notifications::send_native(
                    app.handle(),
                    "StockWatch Local",
                    "Windows qualification notification",
                    false,
                );
                qualification_report.push(format!(
                    "notification={}",
                    if result.is_ok() { "ok" } else { "error" }
                ));
                if let Err(error) = result {
                    qualification_report.push(format!("notification_error={error}"));
                }
            }
            if let Ok(path) = std::env::var("STOCKWATCH_QUALIFICATION_REPORT") {
                let _ = std::fs::write(path, qualification_report.join("\n"));
            }
            if let Ok(mode) = std::env::var("STOCKWATCH_QUALIFICATION_WINDOW") {
                tray::request_qualification_window(app.handle(), &mode, settings.close_behavior);
            }
            if let Some(ms) = std::env::var("STOCKWATCH_QUALIFICATION_EXIT_AFTER_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
            {
                let exit_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                    exit_handle.exit(0);
                });
            }

            let app_handle = app.handle().clone();
            let tray_runtime = setup_runtime.clone();
            let tray_settings = settings_state.clone();
            let refresh_tray: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
                let _ = tray::refresh_native_tray(&app_handle, &tray_runtime, &tray_settings);
            });

            if settings.market_data.refresh_on_startup {
                let initial = service.clone();
                let initial_alerts = alerts.clone();
                let initial_app = app.handle().clone();
                let initial_tray = refresh_tray.clone();
                tauri::async_runtime::spawn(async move {
                    if initial.refresh_once().await.is_ok() {
                        let _ = initial_alerts.evaluate_all(&initial_app);
                    }
                    initial_tray();
                });
            }
            spawn_market_worker(
                service,
                settings_state,
                alerts,
                app.handle().clone(),
                refresh_tray,
            );
            spawn_maintenance_worker(maintenance);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::calculate_portfolio,
            commands::preview_opening_holdings,
            commands::schema_version,
            commands::runtime_snapshot,
            commands::preview_tray_menu,
            commands::dashboard_snapshot,
            commands::request_refresh,
            commands::set_market_symbol_override,
            commands::get_settings,
            commands::save_settings,
            commands::list_alerts,
            commands::save_alert,
            commands::delete_alert,
            commands::alert_history,
            commands::test_notification,
            commands::analytics_snapshot,
            commands::list_cash_flows,
            commands::add_cash_flow,
            commands::delete_cash_flow,
            commands::holding_price_history,
            commands::get_analytics_preferences,
            commands::save_analytics_preferences,
            commands::reliability_status,
            commands::run_maintenance_now,
            commands::create_backup_now,
            commands::stage_restore_backup,
        ])
        .build(tauri::generate_context!())
        .expect("error while building StockWatch Local");

    // Tauri requests process exit when the last WebView is destroyed. StockWatch is tray-first,
    // so a normal window close must keep the Rust backend alive. Explicit app.exit(0) from the
    // tray carries an exit code and remains allowed. Windows shutdown/logoff behavior is part of
    // the final native qualification checklist.
    app.run(|_app_handle, event| {
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
