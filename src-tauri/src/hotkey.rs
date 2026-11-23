//! A global keyboard shortcut that toggles the pop-over. Saved in the
//! database; an empty value turns it off.

use std::str::FromStr;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::store::Store;

const KEY: &str = "hotkey";
pub const DEFAULT: &str = "CommandOrControl+Alt+O";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyStatus {
    /// The saved shortcut, or "" when turned off.
    pub accelerator: String,
    /// Set when the shortcut couldn't be registered (taken by another app,
    /// or a Wayland session, where global shortcuts aren't available).
    pub error: Option<String>,
}

pub struct HotkeyState {
    store: Arc<Mutex<Store>>,
    status: Mutex<HotkeyStatus>,
}

impl HotkeyState {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        let saved = store
            .lock()
            .expect("store lock")
            .setting(KEY)
            .ok()
            .flatten();
        let accelerator = saved.unwrap_or_else(|| DEFAULT.to_string());
        Self {
            store,
            status: Mutex::new(HotkeyStatus {
                accelerator,
                error: None,
            }),
        }
    }
}

/// Checks a shortcut and returns it in a tidy form.
pub fn normalise(accel: &str) -> Result<String, String> {
    let accel = accel.trim();
    if accel.is_empty() {
        return Ok(String::new());
    }
    let parts: Vec<&str> = accel.split('+').map(str::trim).collect();
    if parts.len() < 2 {
        return Err("Use at least one modifier, such as Ctrl+Alt+O.".into());
    }
    Shortcut::from_str(&parts.join("+"))
        .map_err(|e| format!("That isn't a shortcut the app understands: {e}."))?;
    Ok(parts.join("+"))
}

fn register(app: &AppHandle, accel: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    gs.unregister_all().map_err(|e| e.to_string())?;
    if accel.is_empty() {
        return Ok(());
    }
    gs.register(accel)
        .map_err(|e| format!("Couldn't register {accel}: {e}. Another app may be using it."))
}

/// Registers the saved shortcut at start-up. A failure is shown in Settings.
pub fn init(app: &AppHandle) {
    let state = app.state::<HotkeyState>();
    let mut status = state.status.lock().expect("hotkey lock");
    status.error = register(app, &status.accelerator).err();
    if let Some(e) = &status.error {
        log::warn!("global shortcut: {e}");
    }
}

/// The plugin's handler: any of our shortcuts, pressed, toggles the pop-over.
pub fn handle(app: &AppHandle, _shortcut: &Shortcut, state: ShortcutState) {
    if state == ShortcutState::Pressed {
        crate::toggle_popover(app, false);
    }
}

#[tauri::command]
pub fn get_hotkey(state: State<'_, HotkeyState>) -> HotkeyStatus {
    state.status.lock().expect("hotkey lock").clone()
}

#[tauri::command]
pub fn set_hotkey(
    app: AppHandle,
    state: State<'_, HotkeyState>,
    accelerator: String,
) -> Result<HotkeyStatus, String> {
    let accel = normalise(&accelerator)?;
    let mut status = state.status.lock().expect("hotkey lock");
    if let Err(e) = register(&app, &accel) {
        // Put the old one back so a failed change doesn't leave none.
        let _ = register(&app, &status.accelerator);
        return Err(e);
    }
    state
        .store
        .lock()
        .expect("store lock")
        .set_setting(KEY, Some(&accel))
        .map_err(|e| format!("Saving the shortcut: {e}"))?;
    *status = HotkeyStatus {
        accelerator: accel,
        error: None,
    };
    Ok(status.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_shortcuts() {
        assert_eq!(
            normalise(" CommandOrControl + Alt + O ").unwrap(),
            "CommandOrControl+Alt+O"
        );
        assert_eq!(normalise("").unwrap(), "");
        assert!(normalise("O").is_err());
        assert!(normalise("Ctrl+Banana").is_err());
        assert!(normalise(DEFAULT).is_ok());
        // Every form the settings page can produce.
        for ok in [
            "Super+F5",
            "Alt+Shift+Space",
            "CommandOrControl+Control+K",
            "Ctrl+`",
            "Alt+Up",
            "Shift+Alt+/",
            "Ctrl+Alt+1",
        ] {
            assert!(normalise(ok).is_ok(), "{ok}");
        }
    }
}
