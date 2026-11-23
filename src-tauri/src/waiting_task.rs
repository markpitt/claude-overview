//! Background task for "What's waiting": Claude Code sessions (from hook
//! markers, rechecked every 30 seconds and whenever a marker changes),
//! message batches and pull requests (every 5 minutes, from the plan).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{Notify, mpsc};

use crate::batches::{self, Batch};
use crate::github::{self, Pr};
use crate::hooks::{self, HookStatus, Marker};
use crate::keys;

const SESSIONS_EVERY: Duration = Duration::from_secs(30);
const NETWORK_EVERY: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source<T> {
    /// A key or token is saved for this source.
    pub configured: bool,
    pub key_hint: Option<String>,
    /// None until the first fetch.
    pub items: Option<Vec<T>>,
    pub error: Option<String>,
    pub fetched_at_ms: i64,
}

impl<T> Default for Source<T> {
    fn default() -> Self {
        Source { configured: false, key_hint: None, items: None, error: None, fetched_at_ms: 0 }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitingSnapshot {
    pub checked: bool,
    pub sessions: Vec<Marker>,
    pub hooks: HookStatus,
    pub batches: Source<Batch>,
    pub prs: Source<Pr>,
}

pub struct WaitingState {
    last: Mutex<WaitingSnapshot>,
    refresh: Notify,
    /// Set by a refresh request so the next pass also fetches batches and PRs.
    network_now: Mutex<bool>,
}

impl WaitingState {
    pub fn new() -> Self {
        WaitingState {
            last: Mutex::new(WaitingSnapshot {
                checked: false,
                sessions: vec![],
                hooks: HookStatus { installed: false, settings_path: None, error: None },
                batches: Source::default(),
                prs: Source::default(),
            }),
            refresh: Notify::new(),
            network_now: Mutex::new(true),
        }
    }

    pub fn refresh(&self) {
        *self.network_now.lock().expect("waiting lock") = true;
        self.refresh.notify_one();
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn get_waiting(state: State<'_, WaitingState>) -> WaitingSnapshot {
    state.last.lock().expect("waiting lock").clone()
}

#[tauri::command]
pub fn refresh_waiting(state: State<'_, WaitingState>) {
    state.refresh();
}

/// Saves (or with None, removes) the batches API key ("api") or the GitHub
/// token ("github") in the OS keychain.
#[tauri::command]
pub async fn set_waiting_key(state: State<'_, WaitingState>, kind: String, value: Option<String>) -> Result<(), String> {
    let value = value.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let name = match kind.as_str() {
        "api" => {
            if let Some(v) = &value {
                keys::check_api_key(v)?;
            }
            keys::API_KEY
        }
        "github" => {
            if let Some(v) = &value {
                keys::check_github_token(v)?;
            }
            keys::GITHUB_TOKEN
        }
        _ => return Err("unknown key".into()),
    };
    tauri::async_runtime::spawn_blocking(move || keys::set(name, value.as_deref()))
        .await
        .map_err(|e| e.to_string())??;
    state.refresh();
    Ok(())
}

/// Adds or removes the Claude Code hooks in Claude Code's settings file.
#[tauri::command]
pub async fn set_hooks(state: State<'_, WaitingState>, on: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || hooks::set_installed(on))
        .await
        .map_err(|e| e.to_string())??;
    state.refresh.notify_one();
    Ok(())
}

async fn read_key(name: &'static str) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || keys::get(name))
        .await
        .unwrap_or_else(|e| Err(e.to_string()))
}

/// Fetches one source if its key is set, updating `src` in place.
async fn update<T, F, Fut>(src: &mut Source<T>, key: Result<Option<String>, String>, fetch: F)
where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<T>, String>>,
{
    match key {
        Err(e) => src.error = Some(e),
        Ok(None) => *src = Source::default(),
        Ok(Some(k)) => {
            src.configured = true;
            src.key_hint = Some(keys::hint(&k));
            match fetch(k).await {
                Ok(items) => {
                    src.items = Some(items);
                    src.error = None;
                    src.fetched_at_ms = now_ms();
                }
                Err(e) => {
                    log::warn!("waiting fetch failed: {e}");
                    src.error = Some(e);
                }
            }
        }
    }
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::builder()
            .user_agent(concat!("claude-overview/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .ok();
        let dir = hooks::markers_dir();
        if let Some(d) = &dir {
            let _ = std::fs::create_dir_all(d);
        }
        // Wake when a hook writes or removes a marker.
        let (tx, mut rx) = mpsc::unbounded_channel::<()>();
        let _watcher = dir.as_ref().and_then(|d| {
            let tx = tx.clone();
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if res.is_ok() {
                    let _ = tx.send(());
                }
            })
            .and_then(|mut w| w.watch(d, RecursiveMode::NonRecursive).map(|_| w))
            .map_err(|e| log::warn!("watching hook markers: {e}"))
            .ok()
        });
        let mut last_network: Option<Instant> = None;

        loop {
            let state = app.state::<WaitingState>();
            let network = std::mem::take(&mut *state.network_now.lock().expect("waiting lock"))
                || last_network.is_none_or(|t| t.elapsed() >= NETWORK_EVERY);

            let d2 = dir.clone();
            let (sessions, hook_status) = tauri::async_runtime::spawn_blocking(move || {
                (d2.map(|d| hooks::waiting(&d, now_ms())).unwrap_or_default(), hooks::status())
            })
            .await
            .unwrap_or_else(|_| (vec![], hooks::status()));

            let (mut b, mut p) = {
                let last = state.last.lock().expect("waiting lock");
                (last.batches.clone(), last.prs.clone())
            };
            if network {
                last_network = Some(Instant::now());
                if let Some(c) = &client {
                    let now = now_ms();
                    update(&mut b, read_key(keys::API_KEY).await, |k| async move { batches::fetch(c, &k, now).await }).await;
                    update(&mut p, read_key(keys::GITHUB_TOKEN).await, |k| async move { github::fetch(c, &k).await }).await;
                }
            }

            let snap = {
                let mut last = state.last.lock().expect("waiting lock");
                last.checked = true;
                last.sessions = sessions;
                last.hooks = hook_status;
                last.batches = b;
                last.prs = p;
                last.clone()
            };
            let _ = app.emit("waiting-updated", &snap);

            tokio::select! {
                _ = tokio::time::sleep(SESSIONS_EVERY) => {}
                _ = state.refresh.notified() => {}
                Some(()) = rx.recv() => {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    while rx.try_recv().is_ok() {}
                }
            }
        }
    });
}
