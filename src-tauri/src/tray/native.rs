use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder,
};

use crate::{
    market::MarketDataService,
    runtime::RuntimeState,
    settings::{CloseBehavior, SettingsState, TrayLeftClickAction},
    tray::{build_tray_menu_model, parse_tray_action, TrayAction, TrayMenuEntry, TrayMenuKind},
};

pub const TRAY_ID: &str = "stockwatch-main";
const DASHBOARD_LABEL: &str = "main";
const MINI_LABEL: &str = "mini";

pub fn install_native_tray(
    app: &tauri::App,
    runtime: RuntimeState,
    settings: SettingsState,
) -> tauri::Result<()> {
    let initial = settings.snapshot();
    if !initial.tray.enabled {
        return Ok(());
    }

    let menu = native_menu(app, &build_tray_menu_model(&runtime.snapshot(), &initial))?;
    let menu_runtime = runtime.clone();
    let menu_settings = settings.clone();
    let click_settings = settings.clone();

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(tooltip(&runtime.snapshot()))
        .on_menu_event(move |app, event| {
            if let Some(action) = parse_tray_action(event.id().as_ref()) {
                handle_action(app, action, &menu_runtime, &menu_settings);
            }
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let current = click_settings.snapshot();
                match current.tray.left_click_action {
                    TrayLeftClickAction::OpenDashboard => {
                        request_dashboard(tray.app_handle(), None, current.close_behavior)
                    }
                    TrayLeftClickAction::OpenMiniMode => request_mini(tray.app_handle()),
                    TrayLeftClickAction::None => {}
                }
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

pub fn request_qualification_window<R: Runtime>(
    app: &AppHandle<R>,
    mode: &str,
    close_behavior: CloseBehavior,
) {
    match mode.trim().to_ascii_lowercase().as_str() {
        "dashboard" | "main" => request_dashboard(app, None, close_behavior),
        "mini" => request_mini(app),
        _ => {}
    }
}

pub fn refresh_native_tray<R: Runtime>(
    app: &AppHandle<R>,
    runtime: &RuntimeState,
    settings: &SettingsState,
) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    let current = settings.snapshot();
    let menu = native_menu(app, &build_tray_menu_model(&runtime.snapshot(), &current))?;
    tray.set_menu(Some(menu))?;
    tray.set_tooltip(Some(tooltip(&runtime.snapshot())))?;
    Ok(())
}

fn native_menu<R: Runtime, M: Manager<R>>(
    manager: &M,
    model: &[TrayMenuEntry],
) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(manager)?;
    for entry in model {
        match entry.kind {
            TrayMenuKind::Separator => {
                let item = PredefinedMenuItem::separator(manager)?;
                menu.append(&item)?;
            }
            TrayMenuKind::Display | TrayMenuKind::Action => {
                let item = MenuItem::with_id(
                    manager,
                    &entry.id,
                    &entry.label,
                    entry.enabled,
                    None::<&str>,
                )?;
                menu.append(&item)?;
            }
        }
    }
    Ok(menu)
}

fn handle_action<R: Runtime>(
    app: &AppHandle<R>,
    action: TrayAction,
    runtime: &RuntimeState,
    settings: &SettingsState,
) {
    let current = settings.snapshot();
    match action {
        TrayAction::OpenDashboard => request_dashboard(app, None, current.close_behavior),
        TrayAction::OpenMiniMode => request_mini(app),
        TrayAction::OpenSettings => {
            request_dashboard(app, Some("settings".into()), current.close_behavior)
        }
        TrayAction::OpenAlerts => {
            request_dashboard(app, Some("alerts".into()), current.close_behavior)
        }
        TrayAction::OpenSymbol { symbol } => request_dashboard(
            app,
            Some(format!("holding/{symbol}")),
            current.close_behavior,
        ),
        TrayAction::RefreshNow => {
            let service = app.state::<MarketDataService>().inner().clone();
            let app = app.clone();
            let runtime = runtime.clone();
            let settings = settings.clone();
            tauri::async_runtime::spawn(async move {
                let _ = service.refresh_once().await;
                let _ = refresh_native_tray(&app, &runtime, &settings);
            });
        }
        TrayAction::Exit => app.exit(0),
    }
}

fn request_dashboard<R: Runtime>(
    app: &AppHandle<R>,
    route: Option<String>,
    close_behavior: CloseBehavior,
) {
    // WebviewWindowBuilder documents a Windows deadlock when window creation is performed
    // directly inside synchronous commands/event handlers. Move creation outside the tray event.
    let app = app.clone();
    std::thread::spawn(move || {
        let _ = open_dashboard(&app, route.as_deref(), close_behavior);
    });
}

fn open_dashboard<R: Runtime>(
    app: &AppHandle<R>,
    route: Option<&str>,
    close_behavior: CloseBehavior,
) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(DASHBOARD_LABEL) {
        if let Some(route) = route {
            let safe_route: String = route
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-'))
                .collect();
            if !safe_route.is_empty() {
                let script = format!("window.location.hash = '#/{safe_route}'");
                let _ = window.eval(&script);
            }
        }
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    let url = match route {
        Some(route) => WebviewUrl::App(format!("index.html#/{route}").into()),
        None => WebviewUrl::App("index.html".into()),
    };

    // Dashboard is created on demand. When the user closes it, the window/WebView is destroyed
    // by default, leaving only the Rust tray process alive.
    let window = WebviewWindowBuilder::new(app, DASHBOARD_LABEL, url)
        .title("StockWatch Local")
        .inner_size(1180.0, 760.0)
        .min_inner_size(980.0, 620.0)
        .resizable(true)
        .build()?;

    if close_behavior == CloseBehavior::HideToTray {
        let window_for_close = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window_for_close.hide();
            }
        });
    }
    Ok(())
}

fn request_mini<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    std::thread::spawn(move || {
        let _ = open_mini(&app);
    });
}

fn open_mini<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MINI_LABEL) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, MINI_LABEL, WebviewUrl::App("index.html#/mini".into()))
        .title("StockWatch Quick View")
        .inner_size(360.0, 430.0)
        .min_inner_size(330.0, 360.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()?;
    Ok(())
}

fn tooltip(snapshot: &crate::runtime::RuntimeSnapshot) -> String {
    let portfolio = snapshot
        .portfolio_value
        .map(|v| format!("₹{v:.2}"))
        .unwrap_or_else(|| "—".into());
    let day = snapshot
        .day_pnl
        .map(|v| format!("{}{:.2}", if v >= 0.0 { "+₹" } else { "−₹" }, v.abs()))
        .unwrap_or_else(|| "—".into());
    format!("StockWatch Local\n{portfolio} · {day} today")
}
