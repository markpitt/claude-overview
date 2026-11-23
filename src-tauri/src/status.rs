//! Claude service status from the public Statuspage feed at status.claude.com.
//!
//! No credentials are needed. Three endpoints are read on each poll:
//! `summary.json` (components and unresolved incidents), `incidents.json`
//! (recent history) and `scheduled-maintenances/upcoming.json`.

use serde::{Deserialize, Serialize};

pub const STATUS_BASE: &str = "https://status.claude.com/api/v2";

/// Feed base URL. `CLAUDE_OVERVIEW_STATUS_BASE` overrides it so the UI can be
/// tested against saved fixtures (for example a degraded feed).
pub fn status_base() -> String {
    std::env::var("CLAUDE_OVERVIEW_STATUS_BASE").unwrap_or_else(|_| STATUS_BASE.to_string())
}

/// How serious things are right now. Drives the tray icon colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Everything operational.
    Ok,
    /// Minor incident, degraded performance or maintenance in progress.
    Warn,
    /// Major or critical incident, or a partial/major outage.
    Bad,
    /// The feed could not be read, so the state is not known.
    Unknown,
}

impl Level {
    /// Level for a Statuspage page indicator (`none`, `minor`, `major`, `critical`, `maintenance`).
    pub fn from_indicator(indicator: &str) -> Self {
        match indicator {
            "none" => Level::Ok,
            "minor" | "maintenance" => Level::Warn,
            "major" | "critical" => Level::Bad,
            _ => Level::Unknown,
        }
    }

    /// Level for a component status.
    pub fn from_component(status: &str) -> Self {
        match status {
            "operational" => Level::Ok,
            "degraded_performance" | "under_maintenance" => Level::Warn,
            "partial_outage" | "major_outage" => Level::Bad,
            _ => Level::Unknown,
        }
    }
}

// ---- Raw Statuspage shapes (only the fields we use) ----

#[derive(Debug, Deserialize)]
struct RawSummary {
    status: RawPageStatus,
    components: Vec<RawComponent>,
    #[serde(default)]
    incidents: Vec<RawIncident>,
    #[serde(default)]
    scheduled_maintenances: Vec<RawIncident>,
}

#[derive(Debug, Deserialize)]
struct RawPageStatus {
    indicator: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct RawComponent {
    id: String,
    name: String,
    status: String,
    #[serde(default)]
    group: bool,
    #[serde(default)]
    group_id: Option<String>,
    #[serde(default)]
    position: i64,
}

#[derive(Debug, Deserialize)]
struct RawIncidentList {
    #[serde(default)]
    incidents: Vec<RawIncident>,
}

#[derive(Debug, Deserialize)]
struct RawMaintenanceList {
    #[serde(default)]
    scheduled_maintenances: Vec<RawIncident>,
}

#[derive(Debug, Deserialize)]
struct RawIncident {
    id: String,
    name: String,
    status: String,
    #[serde(default)]
    impact: Option<String>,
    created_at: String,
    #[serde(default)]
    resolved_at: Option<String>,
    #[serde(default)]
    scheduled_for: Option<String>,
    #[serde(default)]
    scheduled_until: Option<String>,
    #[serde(default)]
    shortlink: Option<String>,
    #[serde(default)]
    components: Vec<RawIncidentComponent>,
    #[serde(default)]
    incident_updates: Vec<RawUpdate>,
}

#[derive(Debug, Deserialize)]
struct RawIncidentComponent {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawUpdate {
    body: String,
}

// ---- What the UI receives ----

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub id: String,
    pub name: String,
    pub status: String,
    pub level: Level,
    /// Name of the group this component sits in, if any.
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Incident {
    pub id: String,
    pub name: String,
    pub status: String,
    pub impact: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
    pub scheduled_for: Option<String>,
    pub scheduled_until: Option<String>,
    pub url: Option<String>,
    pub components: Vec<String>,
    /// Text of the latest update, if any.
    pub latest_update: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusSnapshot {
    pub level: Level,
    pub description: String,
    pub components: Vec<Component>,
    pub active_incidents: Vec<Incident>,
    pub recent_incidents: Vec<Incident>,
    pub maintenance: Vec<Incident>,
    /// Unix time in milliseconds of the last successful fetch, set by the poller.
    pub fetched_at_ms: u64,
    /// Set when the last fetch failed; the rest of the snapshot is then the
    /// last good one (or empty).
    pub error: Option<String>,
}

impl StatusSnapshot {
    pub fn unknown(error: impl Into<String>) -> Self {
        StatusSnapshot {
            level: Level::Unknown,
            description: "Status unknown".into(),
            components: vec![],
            active_incidents: vec![],
            recent_incidents: vec![],
            maintenance: vec![],
            fetched_at_ms: 0,
            error: Some(error.into()),
        }
    }
}

fn convert_incident(raw: RawIncident) -> Incident {
    Incident {
        id: raw.id,
        name: raw.name,
        status: raw.status,
        impact: raw.impact.unwrap_or_else(|| "none".into()),
        created_at: raw.created_at,
        resolved_at: raw.resolved_at,
        scheduled_for: raw.scheduled_for,
        scheduled_until: raw.scheduled_until,
        url: raw.shortlink,
        components: raw.components.into_iter().map(|c| c.name).collect(),
        latest_update: raw.incident_updates.into_iter().next().map(|u| u.body),
    }
}

/// Builds a snapshot from the three feed bodies. Pure, so it is unit-tested
/// against saved fixtures.
pub fn parse(
    summary: &str,
    incidents: &str,
    upcoming: &str,
    recent_limit: usize,
) -> Result<StatusSnapshot, serde_json::Error> {
    let summary: RawSummary = serde_json::from_str(summary)?;
    let incidents: RawIncidentList = serde_json::from_str(incidents)?;
    let upcoming: RawMaintenanceList = serde_json::from_str(upcoming)?;

    // Group names, so each component can say which group it is in.
    let groups: Vec<(String, String)> = summary
        .components
        .iter()
        .filter(|c| c.group)
        .map(|c| (c.id.clone(), c.name.clone()))
        .collect();

    let mut raw_components: Vec<RawComponent> =
        summary.components.into_iter().filter(|c| !c.group).collect();
    // Top-level services first, then grouped ones, each in page order.
    raw_components.sort_by_key(|c| (c.group_id.is_some(), c.position));

    let components: Vec<Component> = raw_components
        .into_iter()
        .map(|c| Component {
            level: Level::from_component(&c.status),
            group: c
                .group_id
                .as_ref()
                .and_then(|g| groups.iter().find(|(id, _)| id == g))
                .map(|(_, name)| name.clone()),
            id: c.id,
            name: c.name,
            status: c.status,
        })
        .collect();

    // The page indicator can lag a component outage by a minute or two, so
    // take the worse of the two.
    let worst_component = components
        .iter()
        .map(|c| c.level)
        .filter(|l| *l != Level::Unknown)
        .max()
        .unwrap_or(Level::Ok);
    let page_level = Level::from_indicator(&summary.status.indicator);
    let level = if page_level == Level::Unknown {
        worst_component
    } else {
        page_level.max(worst_component)
    };

    // Maintenance: anything in progress from the summary, plus upcoming.
    let mut maintenance: Vec<Incident> = summary
        .scheduled_maintenances
        .into_iter()
        .map(convert_incident)
        .collect();
    for m in upcoming.scheduled_maintenances.into_iter().map(convert_incident) {
        if !maintenance.iter().any(|x| x.id == m.id) {
            maintenance.push(m);
        }
    }

    Ok(StatusSnapshot {
        level,
        description: summary.status.description,
        components,
        active_incidents: summary.incidents.into_iter().map(convert_incident).collect(),
        recent_incidents: incidents
            .incidents
            .into_iter()
            .take(recent_limit)
            .map(convert_incident)
            .collect(),
        maintenance,
        fetched_at_ms: 0,
        error: None,
    })
}

/// Fetches the three feeds and parses them.
pub async fn fetch(client: &reqwest::Client) -> Result<StatusSnapshot, String> {
    async fn get(client: &reqwest::Client, path: &str) -> Result<String, String> {
        let url = format!("{}/{path}", status_base());
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("{path}: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("{path}: HTTP {}", resp.status()));
        }
        resp.text().await.map_err(|e| format!("{path}: {e}"))
    }

    let (summary, incidents, upcoming) = tokio::join!(
        get(client, "summary.json"),
        get(client, "incidents.json"),
        get(client, "scheduled-maintenances/upcoming.json"),
    );
    parse(&summary?, &incidents?, &upcoming?, 10).map_err(|e| format!("parse: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUMMARY: &str = include_str!("../tests/fixtures/summary.json");
    const SUMMARY_DEGRADED: &str = include_str!("../tests/fixtures/summary-degraded.json");
    const INCIDENTS: &str = include_str!("../tests/fixtures/incidents.json");
    const UPCOMING: &str = include_str!("../tests/fixtures/upcoming.json");

    #[test]
    fn healthy_feed_is_ok() {
        let s = parse(SUMMARY, INCIDENTS, UPCOMING, 10).unwrap();
        assert_eq!(s.level, Level::Ok);
        assert_eq!(s.description, "All Systems Operational");
        assert!(s.active_incidents.is_empty());
        // Group rows are folded into their children, not listed.
        assert!(s.components.iter().all(|c| c.name != "Claude on Cloud Platforms"));
        assert!(s.components.iter().any(|c| c.name == "Claude Code"));
        let aws = s
            .components
            .iter()
            .find(|c| c.name == "Claude Platform on AWS")
            .unwrap();
        assert_eq!(aws.group.as_deref(), Some("Claude on Cloud Platforms"));
    }

    #[test]
    fn recent_incidents_are_limited_and_carry_links() {
        let s = parse(SUMMARY, INCIDENTS, UPCOMING, 3).unwrap();
        assert_eq!(s.recent_incidents.len(), 3);
        let first = &s.recent_incidents[0];
        assert!(first.url.as_deref().unwrap_or("").starts_with("https://"));
        assert!(!first.components.is_empty());
        assert!(first.latest_update.is_some());
    }

    #[test]
    fn degraded_feed_is_bad_with_active_incident() {
        let s = parse(SUMMARY_DEGRADED, INCIDENTS, UPCOMING, 10).unwrap();
        assert_eq!(s.level, Level::Bad);
        assert_eq!(s.active_incidents.len(), 1);
        let cc = s.components.iter().find(|c| c.name == "Claude Code").unwrap();
        assert_eq!(cc.level, Level::Bad);
    }

    #[test]
    fn component_outage_beats_a_lagging_page_indicator() {
        let lagging = SUMMARY_DEGRADED.replace("\"major\"", "\"none\"");
        let s = parse(&lagging, INCIDENTS, UPCOMING, 10).unwrap();
        assert_eq!(s.level, Level::Bad);
    }

    #[test]
    fn levels_map_as_documented() {
        assert_eq!(Level::from_indicator("none"), Level::Ok);
        assert_eq!(Level::from_indicator("maintenance"), Level::Warn);
        assert_eq!(Level::from_indicator("critical"), Level::Bad);
        assert_eq!(Level::from_indicator("???"), Level::Unknown);
        assert_eq!(Level::from_component("under_maintenance"), Level::Warn);
        assert_eq!(Level::from_component("major_outage"), Level::Bad);
    }
}
