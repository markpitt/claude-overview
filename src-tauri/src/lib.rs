//! Claude Overview: a tray app for Claude usage, plan, waiting work and status.
//!
//! Rust owns every network call, file read and (from milestone 3) every key. The React
//! UI in the two webviews only renders what Rust emits and sends back commands.

mod cc_service;
mod cc_task;
mod claude_code;
mod status;
mod store;
mod tray_icon;

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use status::{Level, StatusSnapshot};
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};
use tokio::sync::Notify;

const TRAY_ID: &str = "main";
const MAIN: &str = "main";
const POPOVER: &str = "popover";
/// Status feed refresh interval, from the plan (decision 6).
const STATUS_EVERY: Duration = Duration::from_secs(120);
/// First retry after a failed fetch; doubles up to STATUS_EVERY.
const RETRY_FIRST: Duration = Duration::from_secs(15);
/// Argument passed by the autostart entry so the app starts in the tray.
pub const HIDDEN_ARG: &str = "--hidden";
/// Opens (or toggles) the pop-over. On Linux, where the tray sends no click
/// events, bind a desktop shortcut to `claude-overview --popover`.
pub const POPOVER_ARG: &str = "--popover";

struct AppState {
    status: Mutex<StatusSnapshot>,
    refresh: Notify,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---- Windows ----

fn show_window(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Pop-over size in logical pixels; keep in step with tauri.conf.json.
const POPOVER_SIZE: (f64, f64) = (380.0, 520.0);

/// Top-right corner of the monitor's work area (clear of panels and bars).
/// Worked out by hand because the positioner plugin reads the window size
/// before Linux has mapped it and so places it off-screen.
fn place_top_right(w: &tauri::WebviewWindow) {
    let monitor = w
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| w.primary_monitor().ok().flatten());
    let Some(m) = monitor else { return };
    let scale = m.scale_factor();
    let area = m.work_area();
    let margin = 8.0 * scale;
    let width = POPOVER_SIZE.0 * scale;
    let x = area.position.x as f64 + area.size.width as f64 - width - margin;
    let y = area.position.y as f64 + margin;
    let _ = w.set_position(tauri::PhysicalPosition::new(x.max(0.0), y.max(0.0)));
}

fn toggle_popover(app: &AppHandle, from_tray_click: bool) {
    let Some(w) = app.get_webview_window(POPOVER) else {
        return;
    };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    // Show first: on Linux the window has no real size until it is mapped,
    // and placing it before that puts it off-screen.
    let _ = w.show();
    // Next to the tray where the OS reports a tray position (Windows, macOS).
    // Linux sends no tray click events, so there it opens top right.
    let placed = from_tray_click && w.move_window(Position::TrayBottomCenter).is_ok();
    if !placed {
        place_top_right(&w);
    }
    let _ = w.set_focus();
}

// ---- Commands ----

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> StatusSnapshot {
    state.status.lock().expect("status lock").clone()
}

#[tauri::command]
fn refresh_status(state: State<'_, AppState>) {
    state.refresh.notify_one();
}

#[tauri::command]
fn show_main(app: AppHandle) {
    if let Some(p) = app.get_webview_window(POPOVER) {
        let _ = p.hide();
    }
    show_window(&app, MAIN);
}

#[tauri::command]
fn hide_popover(app: AppHandle) {
    if let Some(p) = app.get_webview_window(POPOVER) {
        let _ = p.hide();
    }
}

// ---- Status poller ----

fn apply_status(app: &AppHandle, snap: &StatusSnapshot) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(tray_icon::icon(snap.level)));
        let mut tip = tray_icon::tooltip(snap.level, &snap.description);
        if snap.error.is_some() && snap.level != Level::Unknown {
            tip.push_str(" (stale)");
        }
        let _ = tray.set_tooltip(Some(tip));
    }
    let _ = app.emit("status-updated", snap);
}

fn spawn_status_poller(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let client = match reqwest::Client::builder()
            .user_agent(concat!("claude-overview/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let snap = StatusSnapshot::unknown(format!("HTTP client: {e}"));
                apply_status(&app, &snap);
                return;
            }
        };
        let mut failures: u32 = 0;
        loop {
            let result = status::fetch(&client).await;
            let snap = {
                let state = app.state::<AppState>();
                let mut current = state.status.lock().expect("status lock");
                match result {
                    Ok(mut fresh) => {
                        failures = 0;
                        fresh.fetched_at_ms = now_ms();
                        *current = fresh;
                    }
                    Err(e) => {
                        failures += 1;
                        log::warn!("status fetch failed: {e}");
                        // Keep the last good data, marked stale.
                        if current.fetched_at_ms == 0 {
                            *current = StatusSnapshot::unknown(e);
                        } else {
                            current.error = Some(e);
                        }
                    }
                }
                current.clone()
            };
            apply_status(&app, &snap);

            let wait = if failures == 0 {
                STATUS_EVERY
            } else {
                (RETRY_FIRST * 2u32.saturating_pow(failures - 1)).min(STATUS_EVERY)
            };
            let state = app.state::<AppState>();
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = state.refresh.notified() => {}
            }
        }
    });
}

// ---- Tray ----

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let summary = MenuItemBuilder::with_id("summary", "Show summary").build(app)?;
    let open = MenuItemBuilder::with_id("open", "Open Claude Overview").build(app)?;
    let refresh = MenuItemBuilder::with_id("refresh", "Refresh now").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&summary, &open, &refresh])
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&quit)
        .build()?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_icon::icon(Level::Unknown))
        .tooltip("Claude: checking status…")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "summary" => toggle_popover(app, false),
            "open" => show_main(app.clone()),
            "refresh" => app.state::<AppState>().refresh.notify_one(),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_popover(tray.app_handle(), true);
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // A second launch drives the running copy instead of starting another.
            if args.iter().any(|a| a == POPOVER_ARG) {
                toggle_popover(app, false);
            } else {
                show_window(app, MAIN);
            }
        }));
    }

    let app = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
        .manage(AppState {
            status: Mutex::new(StatusSnapshot::unknown("Not checked yet")),
            refresh: Notify::new(),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            refresh_status,
            show_main,
            hide_popover,
            cc_task::get_claude_code,
            cc_task::refresh_claude_code,
            cc_task::save_cc_settings
        ])
        .setup(|app| {
            // A tray app: no Dock icon on macOS.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Local cache: Claude Code usage, read positions and settings.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store = store::Store::open(&data_dir.join("overview.db"))
                .map_err(|e| format!("opening the local database: {e}"))?;
            app.manage(cc_task::CcState::new(store));

            build_tray(app.handle())?;
            spawn_status_poller(app.handle().clone());
            cc_task::spawn(app.handle().clone());

            // Started by hand: show the window, since on GNOME without the
            // AppIndicator extension there is no tray icon to click.
            let args: Vec<String> = std::env::args().collect();
            if args.iter().any(|a| a == POPOVER_ARG) {
                toggle_popover(app.handle(), false);
            } else if !args.iter().any(|a| a == HIDDEN_ARG) {
                show_window(app.handle(), MAIN);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing the main window hides it; Quit is in the tray menu.
            WindowEvent::CloseRequested { api, .. } if window.label() == MAIN => {
                api.prevent_close();
                let _ = window.hide();
            }
            // The pop-over behaves like a menu: it goes when focus leaves.
            WindowEvent::Focused(false) if window.label() == POPOVER => {
                let _ = window.hide();
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building Claude Overview");

    app.run(|_app, event| {
        // Keep running in the tray when every window is hidden.
        if let RunEvent::ExitRequested { api, code: None, .. } = event {
            api.prevent_exit();
        }
    });
}
