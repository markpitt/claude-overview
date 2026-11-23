//! Claude Code sessions waiting for you, through Claude Code hooks.
//!
//! Claude Code runs `claude-overview --hook` on three events, passing JSON
//! on stdin. The app is not started for this: `run_cli` writes or removes a
//! small marker file and exits.
//!
//! - `Notification` (permission prompt, idle prompt, input needed): marker.
//! - `Stop` (Claude finished its turn): marker.
//! - `SessionEnd`: marker removed.
//!
//! A marker counts only while the session's transcript hasn't changed since
//! it was written, so replying or approving clears it without another hook.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::claude_code::project_name;

pub const HOOK_ARG: &str = "--hook";
const IDENTIFIER: &str = "dev.markpitt.claudeoverview";
/// Claude Code may write a line or two just after the hook; ignore those.
const GRACE_MS: i64 = 5_000;
/// Markers older than this are dropped.
const MAX_AGE_MS: i64 = 7 * 86_400_000;
/// Events this app hooks.
const EVENTS: [&str; 3] = ["Notification", "Stop", "SessionEnd"];

/// Where markers live: `<app data>/waiting`, the same folder Tauri's
/// `app_data_dir` gives the running app.
pub fn markers_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join(IDENTIFIER).join("waiting"))
}

#[derive(Debug, Deserialize)]
struct HookInput {
    session_id: String,
    #[serde(default)]
    transcript_path: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    hook_event_name: String,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    notification_type: Option<String>,
    #[serde(default)]
    last_assistant_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Marker {
    pub session_id: String,
    pub project: String,
    pub cwd: Option<String>,
    /// "permission", "question", "idle" or "finished".
    pub kind: String,
    pub message: Option<String>,
    pub transcript_path: Option<String>,
    pub at_ms: i64,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Session ids become file names, so keep only safe characters.
fn file_name(session_id: &str) -> Option<String> {
    let clean: String = session_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(100)
        .collect();
    (!clean.is_empty()).then(|| format!("{clean}.json"))
}

fn short(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>().trim_end())
    }
}

/// What a hook event means for the marker: Some to write, None to remove.
/// Err when the event should be ignored.
fn marker_for(input: &HookInput, now: i64) -> Result<Option<Marker>, ()> {
    let (kind, message) = match input.hook_event_name.as_str() {
        "SessionEnd" => return Ok(None),
        "Stop" => ("finished", input.last_assistant_message.as_deref().map(|m| short(m, 200))),
        "Notification" => {
            let kind = match input.notification_type.as_deref() {
                Some("permission_prompt") => "permission",
                Some("elicitation_dialog" | "elicitation_url_dialog" | "agent_needs_input") => "question",
                Some("idle_prompt") | None => "idle",
                Some(_) => return Err(()),
            };
            (kind, input.message.as_deref().map(|m| short(m, 200)))
        }
        _ => return Err(()),
    };
    Ok(Some(Marker {
        session_id: input.session_id.clone(),
        project: input.cwd.as_deref().map(project_name).unwrap_or_default(),
        cwd: input.cwd.clone(),
        kind: kind.into(),
        message,
        transcript_path: input.transcript_path.clone(),
        at_ms: now,
    }))
}

/// Applies one hook event to the markers folder.
fn apply(dir: &Path, stdin: &str, now: i64) -> std::io::Result<()> {
    let Ok(input) = serde_json::from_str::<HookInput>(stdin) else { return Ok(()) };
    let Some(name) = file_name(&input.session_id) else { return Ok(()) };
    let path = dir.join(&name);
    match marker_for(&input, now) {
        Err(()) => Ok(()),
        Ok(None) => match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        },
        Ok(Some(m)) => {
            std::fs::create_dir_all(dir)?;
            let tmp = dir.join(format!("{name}.tmp"));
            std::fs::write(&tmp, serde_json::to_vec(&m).map_err(std::io::Error::other)?)?;
            std::fs::rename(&tmp, &path)
        }
    }
}

/// Entry point for `claude-overview --hook`. Never fails loudly: a broken
/// hook must not get in the way of Claude Code.
pub fn run_cli() {
    let mut stdin = String::new();
    if std::io::stdin().take(1 << 20).read_to_string(&mut stdin).is_err() {
        return;
    }
    if let Some(dir) = markers_dir() {
        let _ = apply(&dir, &stdin, now_ms());
    }
}

fn modified_ms(path: &Path) -> Option<i64> {
    let t = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(t.duration_since(UNIX_EPOCH).ok()?.as_millis() as i64)
}

/// Sessions waiting for you, newest first. Drops markers whose transcript has
/// moved on, and deletes very old ones.
pub fn waiting(dir: &Path, now: i64) -> Vec<Marker> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut out = vec![];
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let Some(m) = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice::<Marker>(&b).ok()) else {
            continue;
        };
        if now - m.at_ms > MAX_AGE_MS {
            let _ = std::fs::remove_file(&path);
            continue;
        }
        let moved_on = m
            .transcript_path
            .as_deref()
            .and_then(|t| modified_ms(Path::new(t)))
            .is_some_and(|t| t > m.at_ms + GRACE_MS);
        if !moved_on {
            out.push(m);
        }
    }
    out.sort_by_key(|m| std::cmp::Reverse(m.at_ms));
    out
}

// ---- Installing the hooks in Claude Code's settings ----

/// `$CLAUDE_CONFIG_DIR/settings.json`, else `~/.claude/settings.json`.
pub fn settings_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir).join("settings.json"));
    }
    dirs::home_dir().map(|h| h.join(".claude").join("settings.json"))
}

/// The program Claude Code should run: the AppImage itself when running from
/// one (its mounted path changes every launch), else this executable.
pub fn exe_path() -> Option<String> {
    if let Some(p) = std::env::var_os("APPIMAGE").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p).to_string_lossy().into_owned());
    }
    std::env::current_exe().ok().map(|p| p.to_string_lossy().into_owned())
}

/// A hook entry this app added: it runs a program with `--hook`.
fn is_ours(handler: &Value) -> bool {
    handler.get("args").and_then(Value::as_array).is_some_and(|a| a.iter().any(|v| v == HOOK_ARG))
        && handler.get("command").and_then(Value::as_str).is_some_and(|c| c.contains("claude-overview") || c.contains("Claude Overview"))
}

/// Removes this app's handlers, dropping groups and events left empty.
fn strip_ours(settings: &mut Value) {
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else { return };
    for event in EVENTS {
        let Some(groups) = hooks.get_mut(event).and_then(Value::as_array_mut) else { continue };
        for g in groups.iter_mut() {
            if let Some(hs) = g.get_mut("hooks").and_then(Value::as_array_mut) {
                hs.retain(|h| !is_ours(h));
            }
        }
        groups.retain(|g| g.get("hooks").and_then(Value::as_array).is_none_or(|hs| !hs.is_empty()));
        if groups.is_empty() {
            hooks.remove(event);
        }
    }
    if hooks.is_empty() {
        settings.as_object_mut().map(|o| o.remove("hooks"));
    }
}

/// Adds this app's handler to each event (exec form, so paths with spaces
/// need no quoting on any platform).
fn add_ours(settings: &mut Value, exe: &str) -> Result<(), String> {
    strip_ours(settings);
    let obj = settings.as_object_mut().ok_or("Claude Code's settings file isn't a JSON object.")?;
    let hooks = obj.entry("hooks").or_insert_with(|| json!({}));
    let hooks = hooks.as_object_mut().ok_or("\"hooks\" in Claude Code's settings isn't an object.")?;
    for event in EVENTS {
        let groups = hooks.entry(event).or_insert_with(|| json!([]));
        let groups = groups.as_array_mut().ok_or_else(|| format!("\"hooks.{event}\" isn't a list."))?;
        groups.push(json!({ "hooks": [{ "type": "command", "command": exe, "args": [HOOK_ARG], "timeout": 10 }] }));
    }
    Ok(())
}

fn installed_in(settings: &Value, exe: &str) -> bool {
    EVENTS.iter().all(|event| {
        settings
            .pointer(&format!("/hooks/{event}"))
            .and_then(Value::as_array)
            .is_some_and(|groups| {
                groups.iter().any(|g| {
                    g.get("hooks").and_then(Value::as_array).is_some_and(|hs| {
                        hs.iter().any(|h| is_ours(h) && h.get("command").and_then(Value::as_str) == Some(exe))
                    })
                })
            })
    })
}

fn read_settings(path: &Path) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(s) if s.trim().is_empty() => Ok(json!({})),
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("Claude Code's settings file isn't valid JSON ({e}), so it was left alone.")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(format!("Reading {}: {e}", path.display())),
    }
}

fn write_settings(path: &Path, v: &Value) -> Result<(), String> {
    if path.exists() {
        let backup = path.with_extension("json.claude-overview.bak");
        std::fs::copy(path, &backup).map_err(|e| format!("Backing up {}: {e}", path.display()))?;
    } else if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut text = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    text.push('\n');
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("Writing {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Writing {}: {e}", path.display()))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookStatus {
    pub installed: bool,
    pub settings_path: Option<String>,
    pub error: Option<String>,
}

pub fn status() -> HookStatus {
    let path = settings_path();
    let result = match (&path, exe_path()) {
        (Some(p), Some(exe)) => read_settings(p).map(|v| installed_in(&v, &exe)),
        _ => Err("No home folder found.".into()),
    };
    HookStatus {
        installed: *result.as_ref().unwrap_or(&false),
        settings_path: path.map(|p| p.to_string_lossy().into_owned()),
        error: result.err(),
    }
}

/// Adds (or with `on` false, removes) the hooks. Backs the file up first.
pub fn set_installed(on: bool) -> Result<(), String> {
    let path = settings_path().ok_or("No home folder found.")?;
    let mut v = read_settings(&path)?;
    if on {
        add_ours(&mut v, &exe_path().ok_or("Can't find this program's path.")?)?;
    } else {
        strip_ours(&mut v);
    }
    write_settings(&path, &v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("co-hooks-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn events_write_and_remove_markers() {
        let dir = tmp("markers");
        let n = r#"{"session_id":"abc-1","cwd":"/home/mark/src/tauri-sandbox","hook_event_name":"Notification","message":"Claude needs your permission to use Bash","notification_type":"permission_prompt"}"#;
        apply(&dir, n, 1_000).unwrap();
        let w = waiting(&dir, 2_000);
        assert_eq!(w.len(), 1);
        assert_eq!((w[0].kind.as_str(), w[0].project.as_str()), ("permission", "tauri-sandbox"));

        // Stop replaces it with "finished".
        let s = r#"{"session_id":"abc-1","hook_event_name":"Stop","last_assistant_message":"All tests pass.","stop_hook_active":false}"#;
        apply(&dir, s, 3_000).unwrap();
        assert_eq!(waiting(&dir, 4_000)[0].kind, "finished");

        // Unknown notification types and events are ignored.
        apply(&dir, r#"{"session_id":"x","hook_event_name":"Notification","notification_type":"auth_success"}"#, 1).unwrap();
        apply(&dir, r#"{"session_id":"y","hook_event_name":"PreToolUse"}"#, 1).unwrap();
        apply(&dir, "not json", 1).unwrap();
        assert_eq!(waiting(&dir, 4_000).len(), 1);

        apply(&dir, r#"{"session_id":"abc-1","hook_event_name":"SessionEnd","reason":"clear"}"#, 5_000).unwrap();
        assert!(waiting(&dir, 6_000).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn markers_clear_when_the_transcript_moves_on() {
        let dir = tmp("moved");
        let transcript = dir.join("t.jsonl");
        std::fs::write(&transcript, "{}\n").unwrap();
        let mtime = modified_ms(&transcript).unwrap();
        let ev = |at: i64| {
            let input = format!(
                r#"{{"session_id":"s","hook_event_name":"Stop","transcript_path":{}}}"#,
                serde_json::to_string(&transcript.to_string_lossy()).unwrap()
            );
            apply(&dir, &input, at).unwrap();
        };
        ev(mtime);
        assert_eq!(waiting(&dir, mtime).len(), 1, "written just after the hook: still waiting");
        ev(mtime - 60_000);
        assert!(waiting(&dir, mtime).is_empty(), "transcript changed a minute after the marker");
        ev(mtime - MAX_AGE_MS - 10);
        assert!(waiting(&dir, mtime).is_empty());
        assert!(!dir.join("s.json").exists(), "old marker deleted");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(file_name("../../etc/passwd").as_deref(), Some("etcpasswd.json"));
        assert_eq!(file_name("///"), None);
    }

    #[test]
    fn installs_alongside_existing_hooks_and_removes_cleanly() {
        let exe = "/opt/Claude Overview/claude-overview";
        let mut v: Value = serde_json::from_str(
            r#"{"model":"opus","hooks":{"Stop":[{"hooks":[{"type":"command","command":"notify-send done"}]}]}}"#,
        )
        .unwrap();
        let before = v.clone();
        add_ours(&mut v, exe).unwrap();
        assert!(installed_in(&v, exe));
        assert!(!installed_in(&v, "/elsewhere/claude-overview"));
        // Installing twice doesn't duplicate.
        add_ours(&mut v, exe).unwrap();
        assert_eq!(v.pointer("/hooks/Stop").unwrap().as_array().unwrap().len(), 2);
        assert_eq!(v["hooks"]["Stop"][0]["hooks"][0]["command"], "notify-send done");
        strip_ours(&mut v);
        assert_eq!(v, before);

        let mut empty = json!({});
        add_ours(&mut empty, exe).unwrap();
        strip_ours(&mut empty);
        assert_eq!(empty, json!({}));
        assert!(add_ours(&mut json!([]), exe).is_err());
    }

    #[test]
    fn settings_files_are_backed_up_and_bad_json_left_alone() {
        let dir = tmp("settings");
        let path = dir.join("settings.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert!(read_settings(&path).unwrap_err().contains("left alone"));
        std::fs::write(&path, r#"{"theme":"dark"}"#).unwrap();
        let mut v = read_settings(&path).unwrap();
        add_ours(&mut v, "/usr/bin/claude-overview").unwrap();
        write_settings(&path, &v).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("settings.json.claude-overview.bak")).unwrap(), r#"{"theme":"dark"}"#);
        assert!(installed_in(&read_settings(&path).unwrap(), "/usr/bin/claude-overview"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
