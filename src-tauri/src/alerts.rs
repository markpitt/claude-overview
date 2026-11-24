//! Desktop alerts: a new incident, plan or credit use passing a threshold, and
//! new pull requests. The decisions are pure functions over a small memory that
//! is kept in the database, so a restart doesn't repeat an alert.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_notification::NotificationExt;

use crate::api_credit::CreditSummary;
use crate::cc_service::CcSummary;
use crate::github::Pr;
use crate::status::StatusSnapshot;
use crate::store::Store;

const SETTINGS_KEY: &str = "alerts.settings";
const MEMORY_KEY: &str = "alerts.memory";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AlertSettings {
    /// A new incident on status.claude.com.
    pub incidents: bool,
    /// The 5-hour block or the last 7 days passing the threshold of your allowance.
    pub plan: bool,
    /// API spend passing the threshold of the monthly credit.
    pub credit: bool,
    /// A new pull request waiting for your review or opened by Claude.
    pub prs: bool,
    /// Percentage of an allowance that counts as nearly used up.
    pub threshold_pct: u32,
}

impl Default for AlertSettings {
    fn default() -> Self {
        Self {
            incidents: true,
            plan: true,
            credit: true,
            prs: true,
            threshold_pct: 80,
        }
    }
}

impl AlertSettings {
    fn threshold(&self) -> f64 {
        f64::from(self.threshold_pct.clamp(1, 100)) / 100.0
    }
}

/// What has been alerted already.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Memory {
    /// Active incidents already alerted.
    pub incidents: Vec<(String, String)>,
    /// Each threshold alerts once, then re-arms when use drops back below it
    /// (a new block, a new week, a new credit period).
    pub block_armed: bool,
    pub week_armed: bool,
    pub credit_armed: bool,
    /// Pull requests already seen. None until the first list arrives, which is
    /// taken in silently so setting up a token doesn't bring a flood.
    pub prs_seen: Option<Vec<String>>,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            incidents: Vec::new(),
            block_armed: true,
            week_armed: true,
            credit_armed: true,
            prs_seen: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub title: String,
    pub body: String,
}

fn note(title: impl Into<String>, body: impl Into<String>) -> Note {
    Note {
        title: title.into(),
        body: body.into(),
    }
}

// ---- Decisions ----

pub fn incident_notes(mem: &mut Memory, s: &AlertSettings, snap: &StatusSnapshot) -> Vec<Note> {
    // A failed fetch keeps the last good data; don't treat it as news.
    if snap.error.is_some() || snap.fetched_at_ms == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in &snap.active_incidents {
        if !mem.incidents.iter().any(|(id, _)| *id == i.id) {
            mem.incidents.push((i.id.clone(), i.name.clone()));
            if s.incidents {
                let body = i
                    .latest_update
                    .clone()
                    .unwrap_or_else(|| format!("Impact: {}.", i.impact));
                out.push(note(format!("Claude incident: {}", i.name), body));
            }
        }
    }
    let (still, gone): (Vec<_>, Vec<_>) = mem
        .incidents
        .drain(..)
        .partition(|(id, _)| snap.active_incidents.iter().any(|i| i.id == *id));
    mem.incidents = still;
    if s.incidents {
        for (_, name) in gone {
            out.push(note(
                format!("Resolved: {name}"),
                "Claude reports the incident is resolved.",
            ));
        }
    }
    out
}

/// One threshold: alerts on the way up, re-arms below it.
fn crossing(armed: &mut bool, used: f64, limit: f64, threshold: f64) -> bool {
    if limit <= 0.0 || !used.is_finite() {
        return false;
    }
    if used >= limit * threshold {
        std::mem::replace(armed, false)
    } else {
        *armed = true;
        false
    }
}

fn pct(used: f64, limit: f64) -> u32 {
    (used / limit * 100.0).round() as u32
}

pub fn plan_notes(mem: &mut Memory, s: &AlertSettings, cc: &CcSummary) -> Vec<Note> {
    let mut out = Vec::new();
    let t = s.threshold();
    if let Some(limit) = cc.settings.block_limit_usd {
        let used = cc.current_block.as_ref().map_or(0.0, |b| b.cost_usd);
        if crossing(&mut mem.block_armed, used, limit, t) && s.plan {
            out.push(note(
                format!("5-hour block {}% used", pct(used, limit)),
                format!("${used:.2} of your ${limit:.0} estimate. The block resets 5 hours after it started."),
            ));
        }
    }
    if let Some(limit) = cc.settings.week_limit_usd {
        let used = cc.week.cost_usd;
        if crossing(&mut mem.week_armed, used, limit, t) && s.plan {
            out.push(note(
                format!("Last 7 days {}% used", pct(used, limit)),
                format!("${used:.2} of your ${limit:.0} weekly estimate."),
            ));
        }
    }
    out
}

pub fn credit_notes(mem: &mut Memory, s: &AlertSettings, c: &CreditSummary) -> Vec<Note> {
    if crossing(
        &mut mem.credit_armed,
        c.spent_usd,
        c.allowance_usd,
        s.threshold(),
    ) && s.credit
    {
        vec![note(
            format!("API credit {}% used", pct(c.spent_usd, c.allowance_usd)),
            format!("${:.2} of ${:.0} this month.", c.spent_usd, c.allowance_usd),
        )]
    } else {
        Vec::new()
    }
}

pub fn pr_notes(mem: &mut Memory, s: &AlertSettings, prs: &[Pr]) -> Vec<Note> {
    let urls: Vec<String> = prs.iter().map(|p| p.url.clone()).collect();
    let Some(seen) = mem.prs_seen.replace(urls) else {
        return Vec::new();
    };
    if !s.prs {
        return Vec::new();
    }
    let fresh: Vec<&Pr> = prs.iter().filter(|p| !seen.contains(&p.url)).collect();
    match fresh.as_slice() {
        [] => Vec::new(),
        [p] => vec![note(
            if p.reason == "review" {
                "Review requested"
            } else {
                "Pull request from Claude"
            },
            format!("{}#{}: {}", p.repo, p.number, p.title),
        )],
        many => vec![note(
            format!("{} new pull requests", many.len()),
            many.iter()
                .map(|p| format!("{}#{}", p.repo, p.number))
                .collect::<Vec<_>>()
                .join(", "),
        )],
    }
}

// ---- State ----

pub struct AlertState {
    store: Arc<Mutex<Store>>,
    settings: Mutex<AlertSettings>,
    memory: Mutex<Memory>,
}

impl AlertState {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        let (settings, memory) = {
            let st = store.lock().expect("store lock");
            let get = |k: &str| st.setting(k).ok().flatten();
            (
                get(SETTINGS_KEY)
                    .and_then(|v| serde_json::from_str(&v).ok())
                    .unwrap_or_default(),
                get(MEMORY_KEY)
                    .and_then(|v| serde_json::from_str(&v).ok())
                    .unwrap_or_default(),
            )
        };
        Self {
            store,
            settings: Mutex::new(settings),
            memory: Mutex::new(memory),
        }
    }

    /// Runs a decision against the memory, saves it if it changed and shows the notes.
    fn run(&self, app: &AppHandle, decide: impl FnOnce(&mut Memory, &AlertSettings) -> Vec<Note>) {
        let settings = self.settings.lock().expect("alert settings").clone();
        let notes = {
            let mut mem = self.memory.lock().expect("alert memory");
            let before = mem.clone();
            let notes = decide(&mut mem, &settings);
            if *mem != before
                && let Ok(json) = serde_json::to_string(&*mem)
            {
                let _ = self
                    .store
                    .lock()
                    .expect("store lock")
                    .set_setting(MEMORY_KEY, Some(&json));
            }
            notes
        };
        for n in notes {
            show(app, &n);
        }
    }
}

fn show(app: &AppHandle, n: &Note) {
    if let Err(e) = app
        .notification()
        .builder()
        .title(&n.title)
        .body(&n.body)
        .show()
    {
        log::warn!("notification failed: {e}");
    }
}

fn with_state(app: &AppHandle, f: impl FnOnce(&AlertState)) {
    if let Some(state) = app.try_state::<AlertState>() {
        f(&state);
    }
}

pub fn on_status(app: &AppHandle, snap: &StatusSnapshot) {
    with_state(app, |s| s.run(app, |m, set| incident_notes(m, set, snap)));
}

pub fn on_claude_code(app: &AppHandle, cc: &CcSummary) {
    with_state(app, |s| s.run(app, |m, set| plan_notes(m, set, cc)));
}

pub fn on_credit(app: &AppHandle, c: &CreditSummary) {
    with_state(app, |s| s.run(app, |m, set| credit_notes(m, set, c)));
}

pub fn on_prs(app: &AppHandle, prs: &[Pr]) {
    with_state(app, |s| s.run(app, |m, set| pr_notes(m, set, prs)));
}

/// Shown by the app itself, outside the alert rules (an update, say).
pub fn notify(app: &AppHandle, title: &str, body: &str) {
    show(app, &note(title, body));
}

// ---- Commands ----

#[tauri::command]
pub fn get_alert_settings(state: State<'_, AlertState>) -> AlertSettings {
    state.settings.lock().expect("alert settings").clone()
}

#[tauri::command]
pub fn save_alert_settings(
    state: State<'_, AlertState>,
    settings: AlertSettings,
) -> Result<(), String> {
    if !(50..=100).contains(&settings.threshold_pct) {
        return Err("Choose a threshold between 50% and 100%.".into());
    }
    let json = serde_json::to_string(&settings).map_err(|e| e.to_string())?;
    state
        .store
        .lock()
        .expect("store lock")
        .set_setting(SETTINGS_KEY, Some(&json))
        .map_err(|e| format!("Saving alert settings: {e}"))?;
    *state.settings.lock().expect("alert settings") = settings;
    Ok(())
}

#[tauri::command]
pub fn test_alert(app: AppHandle) -> Result<(), String> {
    app.notification()
        .builder()
        .title("Claude Overview")
        .body("Alerts are working. This is how they will look.")
        .show()
        .map_err(|e| format!("The notification couldn't be shown: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{Incident, Level};

    fn incident(id: &str) -> Incident {
        Incident {
            id: id.into(),
            name: format!("Incident {id}"),
            status: "investigating".into(),
            impact: "major".into(),
            created_at: String::new(),
            resolved_at: None,
            scheduled_for: None,
            scheduled_until: None,
            url: None,
            components: vec![],
            latest_update: Some("Looking into it.".into()),
        }
    }

    fn snap(active: Vec<Incident>) -> StatusSnapshot {
        StatusSnapshot {
            level: Level::Bad,
            description: String::new(),
            components: vec![],
            active_incidents: active,
            recent_incidents: vec![],
            maintenance: vec![],
            fetched_at_ms: 1,
            error: None,
        }
    }

    #[test]
    fn incidents_alert_once_and_on_resolution() {
        let s = AlertSettings::default();
        let mut m = Memory::default();
        let n = incident_notes(&mut m, &s, &snap(vec![incident("a")]));
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].title, "Claude incident: Incident a");
        assert!(incident_notes(&mut m, &s, &snap(vec![incident("a")])).is_empty());
        // A failed fetch changes nothing.
        let mut failed = snap(vec![]);
        failed.error = Some("offline".into());
        assert!(incident_notes(&mut m, &s, &failed).is_empty());
        let n = incident_notes(&mut m, &s, &snap(vec![]));
        assert_eq!(n[0].title, "Resolved: Incident a");
        assert!(m.incidents.is_empty());
    }

    #[test]
    fn thresholds_alert_once_then_rearm() {
        let mut armed = true;
        assert!(!crossing(&mut armed, 70.0, 100.0, 0.8));
        assert!(crossing(&mut armed, 81.0, 100.0, 0.8));
        assert!(!crossing(&mut armed, 95.0, 100.0, 0.8));
        assert!(!crossing(&mut armed, 5.0, 100.0, 0.8));
        assert!(crossing(&mut armed, 90.0, 100.0, 0.8));
        assert!(!crossing(&mut armed, 90.0, 0.0, 0.8));
    }

    fn credit(spent: f64) -> CreditSummary {
        let mut c = crate::api_credit::summarise(vec![], 0, 1, None, 100.0);
        c.spent_usd = spent;
        c
    }

    #[test]
    fn credit_respects_the_switch() {
        let mut m = Memory::default();
        let off = AlertSettings {
            credit: false,
            ..Default::default()
        };
        assert!(credit_notes(&mut m, &off, &credit(85.0)).is_empty());
        // Switched off, the crossing is still recorded, so turning it on later doesn't fire late.
        assert!(!m.credit_armed);
        let mut m = Memory::default();
        let n = credit_notes(&mut m, &AlertSettings::default(), &credit(85.0));
        assert_eq!(n[0].title, "API credit 85% used");
    }

    fn pr(n: i64, reason: &str) -> Pr {
        Pr {
            repo: "a/b".into(),
            number: n,
            title: format!("PR {n}"),
            url: format!("https://github.com/a/b/pull/{n}"),
            author: "x".into(),
            draft: false,
            updated_ms: 0,
            reason: reason.into(),
        }
    }

    #[test]
    fn first_pr_list_is_silent_then_new_ones_alert() {
        let s = AlertSettings::default();
        let mut m = Memory::default();
        assert!(pr_notes(&mut m, &s, &[pr(1, "review")]).is_empty());
        assert!(pr_notes(&mut m, &s, &[pr(1, "review")]).is_empty());
        let n = pr_notes(&mut m, &s, &[pr(1, "review"), pr(2, "claude")]);
        assert_eq!(n, vec![note("Pull request from Claude", "a/b#2: PR 2")]);
        let n = pr_notes(&mut m, &s, &[pr(3, "review"), pr(4, "review")]);
        assert_eq!(n[0].title, "2 new pull requests");
    }
}
