//! API credit: spend this credit period from the Usage and Cost Analytics
//! API, against the monthly allowance, with burn rate and a forecast.
//!
//! The cost endpoint lags 4 to 24 hours, so everything here is worked out up
//! to the `data_refreshed_at` time Anthropic reports, not up to now.

use serde::{Deserialize, Serialize};

use crate::claude_code::parse_rfc3339_ms;

pub const API_BASE: &str = "https://api.anthropic.com";
const BETA: &str = "analytics-usage-cost-2026-08-24";
const VERSION: &str = "2026-10-21";
pub const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;

/// API base URL; `CLAUDE_OVERVIEW_API_BASE` points it at a test server.
pub fn api_base() -> String {
    std::env::var("CLAUDE_OVERVIEW_API_BASE")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| API_BASE.to_string())
}

// ---- Dates (UTC, no time zone crate needed) ----

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Civil date for days since 1970-01-01.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { yoe + era * 400 + 1 } else { yoe + era * 400 }, m, d)
}

fn ms_at(y: i64, m: i64, d: i64) -> i64 {
    days_from_civil(y, m, d) * DAY_MS
}

/// "2026-11-17T00:00:00Z".
pub fn rfc3339(ms: i64) -> String {
    let days = ms.div_euclid(DAY_MS);
    let rem = ms.rem_euclid(DAY_MS) / 1000;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem / 60 % 60, rem % 60)
}

/// The credit period holding `now_ms`: from the last reset day (UTC
/// midnight) to the next. `reset_day` is clamped to 1..=28 so every month
/// has it.
pub fn period(now_ms: i64, reset_day: u32) -> (i64, i64) {
    let rd = reset_day.clamp(1, 28) as i64;
    let (y, m, d) = civil_from_days(now_ms.div_euclid(DAY_MS));
    let (sy, sm) = if d >= rd { (y, m) } else if m == 1 { (y - 1, 12) } else { (y, m - 1) };
    let (ey, em) = if sm == 12 { (sy + 1, 1) } else { (sy, sm + 1) };
    (ms_at(sy, sm, rd), ms_at(ey, em, rd))
}

// ---- Response parsing ----

#[derive(Debug, Deserialize)]
struct RawPage {
    #[serde(default)]
    data: Vec<RawRow>,
    #[serde(default)]
    has_more: bool,
    next_page: Option<String>,
    data_refreshed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawRow {
    starting_at: String,
    #[serde(default)]
    group: RawGroup,
    cost: RawCost,
}

#[derive(Debug, Default, Deserialize)]
struct RawGroup {
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCost {
    amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CostRow {
    /// Start of the row's period (a UTC day, or today so far).
    pub day_ms: i64,
    pub model: String,
    pub usd: f64,
}

#[derive(Debug, Default, PartialEq)]
pub struct Page {
    pub rows: Vec<CostRow>,
    pub next_page: Option<String>,
    pub refreshed_ms: Option<i64>,
}

/// Parses one page of `/v1/organizations/analytics/cost`. Amounts are
/// decimal strings in cents.
pub fn parse_page(body: &str) -> Result<Page, String> {
    let raw: RawPage = serde_json::from_str(body).map_err(|e| format!("unexpected cost response: {e}"))?;
    let mut rows = Vec::with_capacity(raw.data.len());
    for r in raw.data {
        let cents: f64 = r.cost.amount.trim().parse().map_err(|_| format!("bad amount {:?}", r.cost.amount))?;
        let day_ms = parse_rfc3339_ms(&r.starting_at).ok_or_else(|| format!("bad time {:?}", r.starting_at))?;
        rows.push(CostRow { day_ms, model: r.group.model.unwrap_or_else(|| "other".into()), usd: cents / 100.0 });
    }
    Ok(Page {
        rows,
        next_page: if raw.has_more { raw.next_page } else { None },
        refreshed_ms: raw.data_refreshed_at.as_deref().and_then(parse_rfc3339_ms),
    })
}

/// The `error.message` of an API error body, if there is one.
fn error_message(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    v.get("error")?.get("message")?.as_str().map(str::to_string)
}

// ---- Summary ----

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApiSettings {
    /// The monthly credit, in USD.
    pub allowance_usd: f64,
    /// Day of the month the credit resets (UTC).
    pub reset_day: u32,
}

impl ApiSettings {
    pub const DEFAULT_ALLOWANCE: f64 = 100.0;
    pub const DEFAULT_RESET_DAY: u32 = 17;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreditSummary {
    pub period_start_ms: i64,
    pub period_end_ms: i64,
    /// Where Anthropic's data ends; spend after this isn't counted yet.
    pub refreshed_ms: Option<i64>,
    pub spent_usd: f64,
    pub allowance_usd: f64,
    /// Average per day over the period so far.
    pub burn_per_day_usd: f64,
    /// Spend at the period end if the burn rate holds.
    pub forecast_usd: f64,
    /// When the allowance runs out at this rate, if before the period ends.
    pub runs_out_ms: Option<i64>,
    pub rows: Vec<CostRow>,
}

pub fn summarise(rows: Vec<CostRow>, start_ms: i64, end_ms: i64, refreshed_ms: Option<i64>, allowance: f64) -> CreditSummary {
    let spent: f64 = rows.iter().map(|r| r.usd).sum();
    // At least an hour, so a fresh period doesn't divide by nearly nothing.
    let upto = refreshed_ms.unwrap_or(start_ms).clamp(start_ms, end_ms);
    let elapsed_days = ((upto - start_ms).max(HOUR_MS)) as f64 / DAY_MS as f64;
    let burn = spent / elapsed_days;
    let left_days = (end_ms - upto) as f64 / DAY_MS as f64;
    let forecast = spent + burn * left_days;
    let runs_out_ms = if burn > 0.0 && forecast > allowance {
        let at = upto + (((allowance - spent).max(0.0) / burn) * DAY_MS as f64) as i64;
        Some(at.min(end_ms))
    } else {
        None
    };
    CreditSummary {
        period_start_ms: start_ms,
        period_end_ms: end_ms,
        refreshed_ms,
        spent_usd: spent,
        allowance_usd: allowance,
        burn_per_day_usd: burn,
        forecast_usd: forecast,
        runs_out_ms,
        rows,
    }
}

// ---- Fetching ----

async fn fetch_range(
    client: &reqwest::Client,
    key: &str,
    from_ms: i64,
    to_ms: i64,
    daily: bool,
) -> Result<Page, String> {
    let base = api_base();
    let mut out = Page::default();
    let mut page: Option<String> = None;
    for _ in 0..20 {
        let mut q: Vec<(&str, String)> = vec![
            ("starting_at", rfc3339(from_ms)),
            ("ending_at", rfc3339(to_ms)),
            ("group_by[]", "model".into()),
            ("limit", "1000".into()),
        ];
        if daily {
            q.push(("time_bucket_width", "1d".into()));
        }
        if let Some(p) = &page {
            q.push(("page", p.clone()));
        }
        let resp = client
            .get(format!("{base}/v1/organizations/analytics/cost"))
            .query(&q)
            .header("x-api-key", key)
            .header("anthropic-version", VERSION)
            .header("anthropic-beta", BETA)
            .send()
            .await
            .map_err(|e| format!("Could not reach the Anthropic API: {}.", e.without_url()))?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| format!("Reading the cost response: {}.", e.without_url()))?;
        if !status.is_success() {
            let detail = error_message(&body).unwrap_or_default();
            return Err(match status.as_u16() {
                401 => format!("The Admin key was rejected. {detail}"),
                403 => format!("The Admin key can't read cost data. {detail}"),
                410 => "The data refreshed mid-read; it will be read again.".into(),
                429 => "Rate limited by the Anthropic API; will retry.".into(),
                s => format!("Anthropic API error {s}. {detail}"),
            }
            .trim()
            .to_string());
        }
        let p = parse_page(&body)?;
        out.rows.extend(p.rows);
        out.refreshed_ms = p.refreshed_ms.or(out.refreshed_ms);
        match p.next_page {
            Some(n) => page = Some(n),
            None => return Ok(out),
        }
    }
    Err("too many pages in the cost response".into())
}

/// Spend this period by UTC day and model: whole days with daily buckets,
/// then today so far as one row per model (daily buckets would need an end
/// in the future).
pub async fn fetch(client: &reqwest::Client, key: &str, now_ms: i64, settings: &ApiSettings) -> Result<CreditSummary, String> {
    let (start, end) = period(now_ms, settings.reset_day);
    let today = now_ms.div_euclid(DAY_MS) * DAY_MS;
    let hour = now_ms.div_euclid(HOUR_MS) * HOUR_MS;
    let mut rows = vec![];
    let mut refreshed = None;
    if today > start {
        let p = fetch_range(client, key, start, today, true).await?;
        rows.extend(p.rows);
        refreshed = p.refreshed_ms;
    }
    if hour > today {
        let p = fetch_range(client, key, today, hour, false).await?;
        rows.extend(p.rows.into_iter().map(|r| CostRow { day_ms: today, ..r }));
        refreshed = p.refreshed_ms.or(refreshed);
    }
    rows.sort_by(|a, b| a.day_ms.cmp(&b.day_ms).then(a.model.cmp(&b.model)));
    Ok(summarise(rows, start, end, refreshed, settings.allowance_usd))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> i64 {
        parse_rfc3339_ms(s).unwrap()
    }

    #[test]
    fn dates_round_trip() {
        for s in ["1970-01-01T00:00:00Z", "2026-02-28T23:59:59Z", "2028-02-29T12:00:00Z", "2026-12-31T00:00:00Z"] {
            assert_eq!(rfc3339(t(s)), s);
        }
    }

    #[test]
    fn period_runs_from_reset_day_to_reset_day() {
        assert_eq!(
            period(t("2026-11-23T20:00:00Z"), 17),
            (t("2026-11-17T00:00:00Z"), t("2026-12-17T00:00:00Z"))
        );
        // Before the reset day: the period began last month.
        assert_eq!(
            period(t("2026-11-16T23:59:00Z"), 17),
            (t("2026-10-17T00:00:00Z"), t("2026-11-17T00:00:00Z"))
        );
        // Across the year end.
        assert_eq!(
            period(t("2027-01-05T00:00:00Z"), 17),
            (t("2026-12-17T00:00:00Z"), t("2027-01-17T00:00:00Z"))
        );
        // On the day itself, and a day the month lacks.
        assert_eq!(period(t("2026-11-17T00:00:00Z"), 17).0, t("2026-11-17T00:00:00Z"));
        assert_eq!(period(t("2026-02-10T00:00:00Z"), 31).0, t("2026-01-28T00:00:00Z"));
    }

    #[test]
    fn parses_the_documented_shape() {
        let p = parse_page(include_str!("../tests/fixtures/cost-page.json")).unwrap();
        assert_eq!(p.rows.len(), 3);
        assert_eq!(p.rows[0], CostRow { day_ms: t("2026-11-17T00:00:00Z"), model: "claude-opus-5-5".into(), usd: 4.128 });
        assert_eq!(p.rows[2].model, "other", "null model");
        assert_eq!(p.next_page.as_deref(), Some("page_2"));
        assert_eq!(p.refreshed_ms, Some(t("2026-11-23T16:02:11Z")));
        assert!(parse_page("{\"data\":[{\"starting_at\":\"x\",\"cost\":{\"amount\":\"1\"}}]}").is_err());
    }

    #[test]
    fn reads_api_error_messages() {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        assert_eq!(error_message(body).as_deref(), Some("invalid x-api-key"));
        assert_eq!(error_message("<html>"), None);
    }

    #[test]
    fn forecasts_from_the_burn_rate() {
        let start = t("2026-11-17T00:00:00Z");
        let end = t("2026-12-17T00:00:00Z");
        let rows = vec![
            CostRow { day_ms: start, model: "a".into(), usd: 20.0 },
            CostRow { day_ms: start + DAY_MS, model: "a".into(), usd: 10.0 },
        ];
        // 30 dollars over 6 days = 5 a day; 24 days left = 150 at the end.
        let s = summarise(rows, start, end, Some(start + 6 * DAY_MS), 100.0);
        assert!((s.burn_per_day_usd - 5.0).abs() < 1e-9);
        assert!((s.forecast_usd - 150.0).abs() < 1e-9);
        // 70 dollars left at 5 a day = 14 days after the data ends.
        assert_eq!(s.runs_out_ms, Some(start + 20 * DAY_MS));

        let calm = summarise(vec![], start, end, Some(start + DAY_MS), 100.0);
        assert_eq!((calm.spent_usd, calm.forecast_usd, calm.runs_out_ms), (0.0, 0.0, None));
        // No data yet: no division by zero.
        let none = summarise(vec![CostRow { day_ms: start, model: "a".into(), usd: 1.0 }], start, end, None, 100.0);
        assert!(none.forecast_usd.is_finite());
    }
}
