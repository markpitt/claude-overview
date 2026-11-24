//! Background task for API credit: reads the Admin key from the keychain,
//! fetches spend every 30 minutes (the cost data lags hours, so more often
//! gains nothing), and keeps the last good result in the database so the
//! panel isn't empty at start.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Notify;

use crate::api_credit::{self, ApiSettings, CreditSummary};
use crate::keys;
use crate::store::Store;

/// Refresh interval, from the plan (decision 6).
const FETCH_EVERY: Duration = Duration::from_secs(30 * 60);
/// First retry after a failure; doubles up to FETCH_EVERY.
const RETRY_FIRST: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiSnapshot {
    /// An Admin key is saved.
    pub configured: bool,
    /// The keychain has been read at least once since start.
    pub checked: bool,
    /// Last four characters of the saved key.
    pub key_hint: Option<String>,
    pub settings: ApiSettings,
    pub summary: Option<CreditSummary>,
    pub fetched_at_ms: i64,
    pub error: Option<String>,
}

pub struct ApiState {
    store: Arc<Mutex<Store>>,
    last: Mutex<ApiSnapshot>,
    refresh: Notify,
}

fn load_settings(store: &Store) -> ApiSettings {
    let get = |k: &str| store.setting(k).ok().flatten();
    ApiSettings {
        allowance_usd: get("api.allowance_usd")
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v > 0.0)
            .unwrap_or(ApiSettings::DEFAULT_ALLOWANCE),
        reset_day: get("api.reset_day")
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|d| (1..=28).contains(d))
            .unwrap_or(ApiSettings::DEFAULT_RESET_DAY),
    }
}

impl ApiState {
    pub fn refresh(&self) {
        self.refresh.notify_one();
    }

    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        let (settings, summary, fetched_at_ms) = {
            let s = store.lock().expect("store lock");
            let summary = s
                .setting("api.last_summary")
                .ok()
                .flatten()
                .and_then(|j| serde_json::from_str::<CreditSummary>(&j).ok());
            let at = s.setting("api.last_fetched_ms").ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0);
            (load_settings(&s), summary, at)
        };
        ApiState {
            store,
            last: Mutex::new(ApiSnapshot {
                configured: false,
                checked: false,
                key_hint: None,
                settings,
                summary,
                fetched_at_ms,
                error: None,
            }),
            refresh: Notify::new(),
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
pub fn get_api_credit(state: State<'_, ApiState>) -> ApiSnapshot {
    state.last.lock().expect("api lock").clone()
}

#[tauri::command]
pub fn refresh_api_credit(state: State<'_, ApiState>) {
    state.refresh.notify_one();
}

#[tauri::command]
pub fn save_api_settings(state: State<'_, ApiState>, settings: ApiSettings) -> Result<(), String> {
    if !(settings.allowance_usd.is_finite() && settings.allowance_usd > 0.0) {
        return Err("The allowance must be more than zero.".into());
    }
    if !(1..=28).contains(&settings.reset_day) {
        return Err("The reset day must be between 1 and 28.".into());
    }
    {
        let store = state.store.lock().expect("store lock");
        store
            .set_setting("api.allowance_usd", Some(&settings.allowance_usd.to_string()))
            .and_then(|_| store.set_setting("api.reset_day", Some(&settings.reset_day.to_string())))
            .map_err(|e| e.to_string())?;
    }
    state.last.lock().expect("api lock").settings = settings;
    state.refresh.notify_one();
    Ok(())
}

/// Saves (or with None, removes) the Admin key in the OS keychain.
#[tauri::command]
pub async fn set_admin_key(state: State<'_, ApiState>, key: Option<String>) -> Result<(), String> {
    let key = key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty());
    if let Some(k) = &key {
        keys::check_admin_key(k)?;
    }
    let k2 = key.clone();
    tauri::async_runtime::spawn_blocking(move || keys::set(keys::ADMIN_KEY, k2.as_deref()))
        .await
        .map_err(|e| e.to_string())??;
    if key.is_none() {
        let mut last = state.last.lock().expect("api lock");
        last.configured = false;
        last.key_hint = None;
        last.summary = None;
        last.error = None;
        let store = state.store.lock().expect("store lock");
        let _ = store.set_setting("api.last_summary", None);
        let _ = store.set_setting("api.last_fetched_ms", None);
    }
    state.refresh.notify_one();
    Ok(())
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::builder()
            .user_agent(concat!("claude-overview/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build();
        let mut failures: u32 = 0;
        loop {
            let key = tauri::async_runtime::spawn_blocking(|| keys::get(keys::ADMIN_KEY))
                .await
                .unwrap_or_else(|e| Err(e.to_string()));
            let state = app.state::<ApiState>();
            let settings = state.last.lock().expect("api lock").settings.clone();

            let outcome: Result<Option<CreditSummary>, String> = match (&key, &client) {
                (Err(e), _) => Err(e.clone()),
                (Ok(None), _) => Ok(None),
                (_, Err(e)) => Err(format!("HTTP client: {e}")),
                (Ok(Some(k)), Ok(c)) => api_credit::fetch(c, k, now_ms(), &settings).await.map(Some),
            };

            let snap = {
                let mut last = state.last.lock().expect("api lock");
                last.configured = matches!(key, Ok(Some(_)));
                last.checked = true;
                last.key_hint = key.as_ref().ok().and_then(|k| k.as_deref()).map(keys::hint);
                match outcome {
                    Ok(Some(summary)) => {
                        failures = 0;
                        last.fetched_at_ms = now_ms();
                        last.error = None;
                        let store = state.store.lock().expect("store lock");
                        if let Ok(j) = serde_json::to_string(&summary) {
                            let _ = store.set_setting("api.last_summary", Some(&j));
                            let _ = store.set_setting("api.last_fetched_ms", Some(&last.fetched_at_ms.to_string()));
                        }
                        last.summary = Some(summary);
                    }
                    Ok(None) => {
                        failures = 0;
                        last.error = None;
                    }
                    Err(e) => {
                        failures += 1;
                        log::warn!("api credit fetch failed: {e}");
                        // Keep the last good figures, marked with the error.
                        last.error = Some(e);
                    }
                }
                last.clone()
            };
            let _ = app.emit("api-credit-updated", &snap);
            if let Some(c) = &snap.summary {
                crate::alerts::on_credit(&app, c);
            }

            let wait = if failures == 0 {
                FETCH_EVERY
            } else {
                (RETRY_FIRST * 2u32.saturating_pow(failures - 1)).min(FETCH_EVERY)
            };
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = state.refresh.notified() => {}
            }
        }
    });
}
