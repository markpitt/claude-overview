//! Message Batches in progress, and those that finished in the last day,
//! from `GET /v1/messages/batches`. Needs an ordinary API key.

use serde::{Deserialize, Serialize};

use crate::api_credit::api_base;
use crate::claude_code::parse_rfc3339_ms;

const VERSION: &str = "2023-06-01";
const RECENT_MS: i64 = 86_400_000;

#[derive(Debug, Deserialize)]
struct RawList {
    #[serde(default)]
    data: Vec<RawBatch>,
    #[serde(default)]
    has_more: bool,
    last_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawBatch {
    id: String,
    processing_status: String,
    request_counts: Counts,
    created_at: String,
    #[serde(default)]
    ended_at: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    #[serde(default)]
    pub processing: i64,
    #[serde(default)]
    pub succeeded: i64,
    #[serde(default)]
    pub errored: i64,
    #[serde(default)]
    pub canceled: i64,
    #[serde(default)]
    pub expired: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Batch {
    pub id: String,
    /// "in_progress", "canceling" or "ended".
    pub status: String,
    pub counts: Counts,
    pub created_ms: i64,
    pub ended_ms: Option<i64>,
    pub expires_ms: Option<i64>,
}

/// Parses one page. Returns the batches and the cursor for the next page.
fn parse_page(body: &str) -> Result<(Vec<Batch>, Option<String>), String> {
    let raw: RawList = serde_json::from_str(body).map_err(|e| format!("unexpected batches response: {e}"))?;
    let batches = raw
        .data
        .into_iter()
        .map(|b| Batch {
            created_ms: parse_rfc3339_ms(&b.created_at).unwrap_or(0),
            ended_ms: b.ended_at.as_deref().and_then(parse_rfc3339_ms),
            expires_ms: b.expires_at.as_deref().and_then(parse_rfc3339_ms),
            id: b.id,
            status: b.processing_status,
            counts: b.request_counts,
        })
        .collect();
    Ok((batches, if raw.has_more { raw.last_id } else { None }))
}

/// Keeps batches still running and those that ended in the last day.
fn relevant(batches: Vec<Batch>, now_ms: i64) -> Vec<Batch> {
    batches
        .into_iter()
        .filter(|b| b.status != "ended" || b.ended_ms.is_some_and(|e| now_ms - e < RECENT_MS))
        .collect()
}

pub async fn fetch(client: &reqwest::Client, key: &str, now_ms: i64) -> Result<Vec<Batch>, String> {
    let base = api_base();
    let mut all = vec![];
    let mut after: Option<String> = None;
    // Newest first; stop once a page is entirely older than the window.
    for _ in 0..5 {
        let mut q = vec![("limit", "100".to_string())];
        if let Some(a) = &after {
            q.push(("after_id", a.clone()));
        }
        let resp = client
            .get(format!("{base}/v1/messages/batches"))
            .query(&q)
            .header("x-api-key", key)
            .header("anthropic-version", VERSION)
            .send()
            .await
            .map_err(|e| format!("Couldn't reach the Anthropic API: {}.", e.without_url()))?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| format!("Reading the batches response: {}.", e.without_url()))?;
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 => "The API key was rejected.".into(),
                403 => "The API key can't list batches.".into(),
                429 => "Rate limited by the Anthropic API; will retry.".into(),
                s => format!("Anthropic API error {s}."),
            });
        }
        let (page, next) = parse_page(&body)?;
        let old = page.iter().all(|b| now_ms - b.created_ms > 30 * RECENT_MS);
        all.extend(page);
        match next {
            Some(n) if !old => after = Some(n),
            _ => break,
        }
    }
    Ok(relevant(all, now_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_keeps_running_and_recent_batches() {
        let body = r#"{"data":[
            {"id":"msgbatch_1","type":"message_batch","processing_status":"in_progress",
             "request_counts":{"processing":90,"succeeded":10,"errored":0,"canceled":0,"expired":0},
             "created_at":"2026-11-23T10:00:00Z","ended_at":null,"expires_at":"2026-11-24T10:00:00Z"},
            {"id":"msgbatch_2","type":"message_batch","processing_status":"ended",
             "request_counts":{"processing":0,"succeeded":50,"errored":2,"canceled":0,"expired":0},
             "created_at":"2026-11-23T08:00:00Z","ended_at":"2026-11-23T09:00:00Z","expires_at":"2026-11-24T08:00:00Z"},
            {"id":"msgbatch_3","type":"message_batch","processing_status":"ended",
             "request_counts":{"processing":0,"succeeded":5,"errored":0,"canceled":0,"expired":0},
             "created_at":"2026-11-20T08:00:00Z","ended_at":"2026-11-20T09:00:00Z","expires_at":"2026-11-21T08:00:00Z"}
          ],"has_more":true,"first_id":"msgbatch_1","last_id":"msgbatch_3"}"#;
        let (page, next) = parse_page(body).unwrap();
        assert_eq!(next.as_deref(), Some("msgbatch_3"));
        assert_eq!(page[0].counts.processing, 90);
        let now = parse_rfc3339_ms("2026-11-23T12:00:00Z").unwrap();
        let keep = relevant(page, now);
        assert_eq!(keep.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["msgbatch_1", "msgbatch_2"]);
        assert!(parse_page("[]").is_err());
    }
}
