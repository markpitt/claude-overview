//! Background task that keeps Claude Code usage current: a file watcher on
//! the transcripts folder for prompt updates, plus a scan every 5 minutes in
//! case the watcher misses something (network drives, sleeping laptops).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{Notify, mpsc};

use crate::cc_service::{self, CcSettings, CcSummary, Health};
use crate::store::Store;

/// Fallback scan interval, from the plan (decision 6).
const SCAN_EVERY: Duration = Duration::from_secs(300);
/// Claude Code writes several lines per response; wait for the burst to end.
const DEBOUNCE: Duration = Duration::from_millis(1500);

pub struct CcState {
    store: Arc<Mutex<Store>>,
    last: Mutex<Option<CcSummary>>,
    refresh: Notify,
    /// Set when the folder setting changes, so the next scan starts over.
    folder_changed: Mutex<bool>,
}

impl CcState {
    pub fn refresh(&self) {
        self.refresh.notify_one();
    }

    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        CcState {
            store,
            last: Mutex::new(None),
            refresh: Notify::new(),
            folder_changed: Mutex::new(false),
        }
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn get_claude_code(state: State<'_, CcState>) -> Option<CcSummary> {
    state.last.lock().expect("cc lock").clone()
}

#[tauri::command]
pub fn refresh_claude_code(state: State<'_, CcState>) {
    state.refresh.notify_one();
}

#[tauri::command]
pub fn save_cc_settings(state: State<'_, CcState>, settings: CcSettings) -> Result<(), String> {
    let store = state.store.lock().expect("store lock");
    let before = CcSettings::load(&store);
    settings.save(&store).map_err(|e| e.to_string())?;
    if CcSettings::load(&store).folder_override != before.folder_override {
        *state.folder_changed.lock().expect("cc lock") = true;
    }
    drop(store);
    state.refresh.notify_one();
    Ok(())
}

/// One scan plus summary, run off the async runtime because SQLite and file
/// reads block.
async fn scan_once(app: &AppHandle, lines: &mut (u64, u64)) -> CcSummary {
    let state = app.state::<CcState>();
    let store = state.store.clone();
    let reset = std::mem::take(&mut *state.folder_changed.lock().expect("cc lock"));
    let (parsed, skipped) = *lines;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut store = store.lock().expect("store lock");
        if reset {
            let _ = store.reset_offsets();
        }
        let settings = CcSettings::load(&store);
        let root = cc_service::folder(&settings);
        let now = now_ms();
        let mut health = Health {
            folder: root.clone(),
            files: 0,
            lines_parsed: parsed,
            lines_skipped: skipped,
            last_scan_ms: now,
            error: None,
        };
        match &root {
            Some(r) if r.is_dir() => match cc_service::scan(&mut store, r) {
                Ok(s) => {
                    health.files = s.files;
                    health.lines_parsed += s.lines_parsed;
                    health.lines_skipped += s.lines_skipped;
                }
                Err(e) => health.error = Some(e.to_string()),
            },
            Some(r) => health.error = Some(format!("Folder not found: {}", r.display())),
            None => health.error = Some("No home folder found".into()),
        }
        let _ = store.prune(now);
        cc_service::summary(&store, now, &health, settings)
    })
    .await;

    match result {
        Ok(summary) => {
            *lines = (summary.lines_parsed, summary.lines_skipped);
            summary
        }
        Err(e) => {
            log::error!("claude code scan panicked: {e}");
            state.last.lock().expect("cc lock").clone().unwrap_or_else(|| {
                let store = state.store.lock().expect("store lock");
                let health = Health {
                    folder: None,
                    files: 0,
                    lines_parsed: 0,
                    lines_skipped: 0,
                    last_scan_ms: now_ms(),
                    error: Some(format!("scan failed: {e}")),
                };
                cc_service::summary(&store, now_ms(), &health, CcSettings::load(&store))
            })
        }
    }
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let (tx, mut rx) = mpsc::unbounded_channel::<()>();
        let mut watcher: Option<(PathBuf, notify::RecommendedWatcher)> = None;
        let mut lines = (0u64, 0u64);

        loop {
            let summary = scan_once(&app, &mut lines).await;

            // (Re)attach the watcher when the folder appears or changes.
            let root = summary.folder.as_ref().map(PathBuf::from);
            let watching = watcher.as_ref().map(|(p, _)| p.clone());
            if root != watching || (watcher.is_none() && summary.folder_exists) {
                watcher = None;
                if let Some(r) = root.filter(|r| r.is_dir()) {
                    let tx = tx.clone();
                    let made = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                        if res.is_ok() {
                            let _ = tx.send(());
                        }
                    })
                    .and_then(|mut w| w.watch(&r, RecursiveMode::Recursive).map(|_| w));
                    match made {
                        Ok(w) => watcher = Some((r, w)),
                        Err(e) => log::warn!("watching {}: {e}", r.display()),
                    }
                }
            }

            let state = app.state::<CcState>();
            *state.last.lock().expect("cc lock") = Some(summary.clone());
            let _ = app.emit("claude-code-updated", &summary);

            tokio::select! {
                _ = tokio::time::sleep(SCAN_EVERY) => {}
                _ = state.refresh.notified() => {}
                Some(()) = rx.recv() => {
                    tokio::time::sleep(DEBOUNCE).await;
                    while rx.try_recv().is_ok() {}
                }
            }
        }
    });
}
