//! Checks GitHub releases for a newer version once a day, and downloads and
//! installs it when asked. Updates are signed; the public key is in
//! tauri.conf.json and the private key is a GitHub Actions secret.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Notify;

const CHECK_EVERY: Duration = Duration::from_secs(24 * 3600);
/// Give the app a moment to settle before the first check.
const FIRST_CHECK: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshot {
    pub current: String,
    /// A newer version, when one is published.
    pub available: Option<String>,
    pub notes: Option<String>,
    pub checked_at_ms: i64,
    pub checking: bool,
    /// Bytes downloaded and the total, while installing.
    pub progress: Option<(u64, Option<u64>)>,
    pub error: Option<String>,
}

pub struct UpdateState {
    last: Mutex<UpdateSnapshot>,
    pending: Mutex<Option<Update>>,
    check: Notify,
}

impl UpdateState {
    pub fn new(current: String) -> Self {
        Self {
            last: Mutex::new(UpdateSnapshot {
                current,
                ..Default::default()
            }),
            pending: Mutex::new(None),
            check: Notify::new(),
        }
    }
}

/// `CLAUDE_OVERVIEW_UPDATE_URL` points the check at a test server.
fn updater(app: &AppHandle) -> tauri_plugin_updater::Result<tauri_plugin_updater::Updater> {
    let mut builder = app.updater_builder();
    if let Ok(url) = std::env::var("CLAUDE_OVERVIEW_UPDATE_URL") {
        let url = url
            .parse()
            .map_err(|_| tauri_plugin_updater::Error::EmptyEndpoints)?;
        builder = builder.endpoints(vec![url])?;
    }
    builder.build()
}

fn now_ms() -> i64 {
    crate::now_ms() as i64
}

fn publish(app: &AppHandle, change: impl FnOnce(&mut UpdateSnapshot)) {
    let state = app.state::<UpdateState>();
    let snap = {
        let mut last = state.last.lock().expect("update lock");
        change(&mut last);
        last.clone()
    };
    let _ = app.emit("update-updated", &snap);
}

/// Plain words for the errors people will actually meet.
fn explain(e: &tauri_plugin_updater::Error) -> String {
    let text = e.to_string();
    if text.contains("404") || text.contains("Could not fetch a valid release JSON") {
        "No published release to update from yet.".into()
    } else {
        format!("Couldn't check for updates: {text}")
    }
}

async fn check_once(app: &AppHandle) {
    publish(app, |s| {
        s.checking = true;
        s.error = None;
    });
    let result = match updater(app) {
        Ok(u) => u.check().await,
        Err(e) => Err(e),
    };
    let state = app.state::<UpdateState>();
    let was = state.last.lock().expect("update lock").available.clone();
    match result {
        Ok(Some(update)) => {
            let version = update.version.clone();
            let notes = update.body.clone();
            *state.pending.lock().expect("update lock") = Some(update);
            if was.as_deref() != Some(version.as_str()) {
                crate::alerts::notify(
                    app,
                    &format!("Claude Overview {version} is available"),
                    "Install it from Settings, Updates.",
                );
            }
            publish(app, |s| {
                s.available = Some(version);
                s.notes = notes;
                s.checking = false;
                s.checked_at_ms = now_ms();
            });
        }
        Ok(None) => {
            *state.pending.lock().expect("update lock") = None;
            publish(app, |s| {
                s.available = None;
                s.notes = None;
                s.checking = false;
                s.checked_at_ms = now_ms();
            });
        }
        Err(e) => {
            log::warn!("update check failed: {e}");
            publish(app, |s| {
                s.checking = false;
                s.checked_at_ms = now_ms();
                s.error = Some(explain(&e));
            });
        }
    }
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<UpdateState>();
        tokio::select! {
            _ = tokio::time::sleep(FIRST_CHECK) => {}
            _ = state.check.notified() => {}
        }
        loop {
            check_once(&app).await;
            tokio::select! {
                _ = tokio::time::sleep(CHECK_EVERY) => {}
                _ = state.check.notified() => {}
            }
        }
    });
}

#[tauri::command]
pub fn get_update(state: State<'_, UpdateState>) -> UpdateSnapshot {
    state.last.lock().expect("update lock").clone()
}

#[tauri::command]
pub fn check_update(state: State<'_, UpdateState>) {
    state.check.notify_one();
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = app
        .state::<UpdateState>()
        .pending
        .lock()
        .expect("update lock")
        .clone()
        .ok_or("No update to install. Check again first.")?;
    publish(&app, |s| {
        s.error = None;
        s.progress = Some((0, None));
    });
    let mut done: u64 = 0;
    let progress_app = app.clone();
    let result = update
        .download_and_install(
            move |chunk, total| {
                done += chunk as u64;
                let d = done;
                publish(&progress_app, |s| s.progress = Some((d, total)));
            },
            || {},
        )
        .await;
    match result {
        Ok(()) => {
            // On Windows the installer has already closed the app by now.
            app.restart();
        }
        Err(e) => {
            let msg = format!("The update didn't install: {e}");
            publish(&app, |s| {
                s.progress = None;
                s.error = Some(msg.clone());
            });
            Err(msg)
        }
    }
}
