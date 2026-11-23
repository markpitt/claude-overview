//! Claude Code usage from the local transcripts in `~/.claude/projects`.
//!
//! The format is undocumented. What this relies on, checked against a real
//! Claude Code 2.1.330 transcript on 23 November 2026:
//! - one JSON object per line; usage lives on lines with `"type": "assistant"`
//! - `message.usage` carries `input_tokens`, `output_tokens`,
//!   `cache_read_input_tokens` and `cache_creation.{ephemeral_5m,ephemeral_1h}_input_tokens`
//! - one API response is written as several lines (one per content block)
//!   with the same `message.id` and `requestId`, so lines must be de-duplicated
//! - `timestamp` (RFC 3339), `cwd`, `sessionId` and `message.model` are present
//!
//! Lines that don't match are counted as skipped, never treated as errors.

use serde::{Deserialize, Serialize};

/// One de-duplicated API response's usage.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageEvent {
    pub message_id: String,
    pub request_id: String,
    pub ts_ms: i64,
    pub model: String,
    pub project: String,
    pub session_id: String,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write_5m: i64,
    pub cache_write_1h: i64,
}

#[derive(Debug, PartialEq)]
pub enum Line {
    Usage(UsageEvent),
    /// A valid line that carries no usage (user turns, attachments, titles…).
    Other,
    /// Not JSON, or an assistant line missing fields we need.
    Skipped,
}

#[derive(Deserialize)]
struct RawLine {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    cwd: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    message: Option<RawMessage>,
}

#[derive(Deserialize)]
struct RawMessage {
    id: Option<String>,
    model: Option<String>,
    usage: Option<RawUsage>,
}

#[derive(Deserialize)]
struct RawUsage {
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
    #[serde(default)]
    cache_read_input_tokens: i64,
    #[serde(default)]
    cache_creation_input_tokens: i64,
    cache_creation: Option<RawCacheCreation>,
}

#[derive(Deserialize)]
struct RawCacheCreation {
    #[serde(default)]
    ephemeral_5m_input_tokens: i64,
    #[serde(default)]
    ephemeral_1h_input_tokens: i64,
}

/// Last path component of the working directory, used as the project name.
pub fn project_name(cwd: &str) -> String {
    cwd.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(cwd)
        .to_string()
}

pub fn parse_line(line: &str) -> Line {
    let line = line.trim();
    if line.is_empty() {
        return Line::Other;
    }
    let Ok(raw) = serde_json::from_str::<RawLine>(line) else {
        return Line::Skipped;
    };
    if raw.kind.as_deref() != Some("assistant") {
        return Line::Other;
    }
    let (Some(msg), Some(ts)) = (raw.message, raw.timestamp) else {
        return Line::Skipped;
    };
    let (Some(id), Some(usage)) = (msg.id, msg.usage) else {
        return Line::Skipped;
    };
    let Some(ts_ms) = parse_rfc3339_ms(&ts) else {
        return Line::Skipped;
    };
    // Synthetic local messages (errors, interrupts) report model "<synthetic>"
    // and zero usage; they cost nothing, so leave them out.
    let model = msg.model.unwrap_or_default();
    if model.is_empty() || model.starts_with('<') {
        return Line::Other;
    }
    let (w5, w1) = match usage.cache_creation {
        Some(c) => (c.ephemeral_5m_input_tokens, c.ephemeral_1h_input_tokens),
        // Older versions only give the total; it was 5-minute caching then.
        None => (usage.cache_creation_input_tokens, 0),
    };
    Line::Usage(UsageEvent {
        message_id: id,
        request_id: raw.request_id.unwrap_or_default(),
        ts_ms,
        model,
        project: raw.cwd.as_deref().map(project_name).unwrap_or_else(|| "unknown".into()),
        session_id: raw.session_id.unwrap_or_default(),
        input: usage.input_tokens,
        output: usage.output_tokens,
        cache_read: usage.cache_read_input_tokens,
        cache_write_5m: w5,
        cache_write_1h: w1,
    })
}

/// Parses `2026-11-23T14:47:23.376Z` (or with an offset) to Unix milliseconds.
/// Small and dependency-free because only this one shape appears.
pub fn parse_rfc3339_ms(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, sec) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let mut i = 19;
    let mut ms = 0i64;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        let frac = &s[start..i];
        let first3: String = frac.chars().chain("000".chars()).take(3).collect();
        ms = first3.parse().ok()?;
    }
    let offset_min = match b.get(i)? {
        b'Z' | b'z' => 0,
        sign @ (b'+' | b'-') => {
            let oh = num(i + 1..i + 3)?;
            let om = num(i + 4..i + 6)?;
            let v = oh * 60 + om;
            if *sign == b'+' { v } else { -v }
        }
        _ => return None,
    };
    // Days from civil (Howard Hinnant's algorithm).
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + h * 3600 + mi * 60 + sec - offset_min * 60;
    Some(secs * 1000 + ms)
}

// ---- Pricing ----

/// USD per million tokens. Claude API list prices as of November 2026.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rates {
    pub input: f64,
    pub output: f64,
    pub cache_write_5m: f64,
    pub cache_write_1h: f64,
    pub cache_read: f64,
}

const fn rates(input: f64, output: f64, cache_read: f64) -> Rates {
    // Cache writes are 1.25x input for 5 minutes and 2x for 1 hour.
    Rates { input, output, cache_write_5m: input * 1.25, cache_write_1h: input * 2.0, cache_read }
}

/// Model id prefix to rates; the first match wins, so longer ids come first.
const PRICES: &[(&str, Rates)] = &[
    ("claude-fable-5-2", rates(8.0, 40.0, 0.20)),
    ("claude-mythos-5-2", rates(8.0, 40.0, 0.20)),
    ("claude-fable-5-1", rates(10.0, 50.0, 1.0)),
    ("claude-mythos-5-1", rates(10.0, 50.0, 1.0)),
    ("claude-fable-5", rates(10.0, 50.0, 1.0)),
    ("claude-opus-5-5", rates(4.0, 20.0, 0.20)),
    ("claude-opus-5", rates(5.0, 25.0, 0.50)),
    ("claude-opus-4", rates(5.0, 25.0, 0.50)),
    ("claude-sonnet-5-5", rates(2.0, 10.0, 0.10)),
    ("claude-sonnet-5", rates(2.0, 10.0, 0.20)),
    ("claude-sonnet-4", rates(3.0, 15.0, 0.30)),
    ("claude-haiku-5-5", rates(0.10, 0.50, 0.01)),
    ("claude-haiku-4", rates(1.0, 5.0, 0.10)),
];

pub fn rates_for(model: &str) -> Option<Rates> {
    PRICES.iter().find(|(p, _)| model.starts_with(p)).map(|(_, r)| *r)
}

/// What this usage would have cost on the Claude API, or None for an
/// unknown model.
pub fn api_cost_usd(e: &UsageEvent) -> Option<f64> {
    let r = rates_for(&e.model)?;
    Some(
        (e.input as f64 * r.input
            + e.output as f64 * r.output
            + e.cache_read as f64 * r.cache_read
            + e.cache_write_5m as f64 * r.cache_write_5m
            + e.cache_write_1h as f64 * r.cache_write_1h)
            / 1_000_000.0,
    )
}

// ---- 5-hour blocks ----

pub const BLOCK_MS: i64 = 5 * 3600 * 1000;
const HOUR_MS: i64 = 3600 * 1000;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub start_ms: i64,
    pub end_ms: i64,
    pub last_activity_ms: i64,
    pub tokens: i64,
    pub cost_usd: f64,
    pub messages: i64,
}

/// Groups events (sorted by time) into 5-hour usage blocks. A block starts
/// at the hour of the first message after the previous block ended. This is
/// an estimate of Anthropic's window, which isn't published: it is the same
/// convention community tools use.
pub fn blocks(events: &[(i64, i64, f64)]) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    for &(ts, tokens, cost) in events {
        match out.last_mut() {
            Some(b) if ts < b.end_ms => {
                b.tokens += tokens;
                b.cost_usd += cost;
                b.messages += 1;
                b.last_activity_ms = b.last_activity_ms.max(ts);
            }
            _ => {
                let start = ts - ts.rem_euclid(HOUR_MS);
                out.push(Block {
                    start_ms: start,
                    end_ms: start + BLOCK_MS,
                    last_activity_ms: ts,
                    tokens,
                    cost_usd: cost,
                    messages: 1,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../tests/fixtures/claude-code-sample.jsonl");

    fn events() -> (Vec<UsageEvent>, usize) {
        let mut seen = std::collections::HashSet::new();
        let mut out = vec![];
        let mut skipped = 0;
        for l in SAMPLE.lines() {
            match parse_line(l) {
                Line::Usage(e) => {
                    if seen.insert((e.message_id.clone(), e.request_id.clone())) {
                        out.push(e)
                    }
                }
                Line::Skipped => skipped += 1,
                Line::Other => {}
            }
        }
        (out, skipped)
    }

    #[test]
    fn duplicates_are_dropped_and_bad_lines_counted() {
        let (ev, skipped) = events();
        // 4 distinct responses; msg_A appears on two lines; one synthetic.
        assert_eq!(ev.len(), 4);
        assert_eq!(skipped, 1);
        assert_eq!(ev[0].project, "claude-overview");
        assert_eq!(ev[0].cache_write_1h, 50386);
        assert_eq!(ev[2].model, "claude-sonnet-5-5");
    }

    #[test]
    fn old_lines_without_cache_breakdown_count_as_5m() {
        let (ev, _) = events();
        let old = ev.iter().find(|e| e.message_id == "msg_D").unwrap();
        assert_eq!(old.cache_write_5m, 1000);
        assert_eq!(old.cache_write_1h, 0);
    }

    #[test]
    fn opus_5_5_price_matches_claude_codes_own_figure() {
        // Totals and the $9.9026756 cost Claude Code itself recorded for one
        // real session (cost-state line), 23 November 2026.
        let e = UsageEvent {
            message_id: "x".into(),
            request_id: "y".into(),
            ts_ms: 0,
            model: "claude-opus-5-5".into(),
            project: "p".into(),
            session_id: "s".into(),
            input: 233,
            output: 82_006,
            cache_read: 29_366_558,
            cache_write_5m: 0,
            cache_write_1h: 298_539,
        };
        let cost = api_cost_usd(&e).unwrap();
        assert!((cost - 9.902_675_6).abs() < 1e-6, "{cost}");
    }

    #[test]
    fn unknown_models_have_no_price() {
        assert!(rates_for("claude-opus-5-5-20270101").is_some());
        assert!(rates_for("gpt-9").is_none());
        assert_eq!(rates_for("claude-sonnet-5-5").unwrap().cache_read, 0.10);
        assert_eq!(rates_for("claude-sonnet-5").unwrap().cache_read, 0.20);
    }

    #[test]
    fn timestamps_parse() {
        assert_eq!(parse_rfc3339_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339_ms("2026-11-23T14:47:23.376Z"), Some(1_795_445_243_376));
        assert_eq!(
            parse_rfc3339_ms("2026-11-23T15:47:23.376+01:00"),
            Some(1_795_445_243_376)
        );
        assert_eq!(parse_rfc3339_ms("not a date"), None);
    }

    #[test]
    fn blocks_start_on_the_hour_and_last_five_hours() {
        let h = HOUR_MS;
        let ev = vec![
            (10 * h + 20 * 60_000, 100, 1.0), // 10:20 opens 10:00-15:00
            (14 * h + 59 * 60_000, 50, 0.5),  // still inside
            (15 * h + 5 * 60_000, 10, 0.1),   // 15:05 opens 15:00-20:00
        ];
        let b = blocks(&ev);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].start_ms, 10 * h);
        assert_eq!(b[0].end_ms, 15 * h);
        assert_eq!(b[0].tokens, 150);
        assert_eq!(b[0].messages, 2);
        assert_eq!(b[1].start_ms, 15 * h);
    }

    #[test]
    fn project_names_come_from_the_last_path_part() {
        assert_eq!(project_name("/home/mark/src/claude-overview"), "claude-overview");
        assert_eq!(project_name("C:\\Users\\Mark\\src\\tiny\\"), "tiny");
    }
}
