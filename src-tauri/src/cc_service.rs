//! Reads new lines from Claude Code transcripts into the store, and builds
//! the summary the UI shows. File watching lives in lib.rs.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::claude_code::{self, Block, Line};
use crate::store::{HourRow, KEEP_DAYS, Store};

const DAY_MS: i64 = 86_400_000;

/// Settings for this panel, stored as plain values in SQLite.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CcSettings {
    /// Folder holding the per-project transcript folders. None = default.
    pub folder_override: Option<String>,
    /// Your own estimate of a 5-hour block's allowance, in API-price dollars.
    pub block_limit_usd: Option<f64>,
    /// Your own estimate of the weekly allowance, in API-price dollars.
    pub week_limit_usd: Option<f64>,
    /// What the Max plan costs per month, to compare with the API price.
    pub plan_price_usd: Option<f64>,
}

impl CcSettings {
    pub fn load(store: &Store) -> Self {
        let num = |k: &str| store.setting(k).ok().flatten().and_then(|v| v.parse::<f64>().ok());
        CcSettings {
            folder_override: store.setting("cc.folder").ok().flatten().filter(|s| !s.is_empty()),
            block_limit_usd: num("cc.block_limit_usd"),
            week_limit_usd: num("cc.week_limit_usd"),
            plan_price_usd: num("cc.plan_price_usd"),
        }
    }

    pub fn save(&self, store: &Store) -> rusqlite::Result<()> {
        let num = |v: Option<f64>| v.filter(|x| x.is_finite() && *x > 0.0).map(|x| x.to_string());
        store.set_setting("cc.folder", self.folder_override.as_deref().filter(|s| !s.trim().is_empty()))?;
        store.set_setting("cc.block_limit_usd", num(self.block_limit_usd).as_deref())?;
        store.set_setting("cc.week_limit_usd", num(self.week_limit_usd).as_deref())?;
        store.set_setting("cc.plan_price_usd", num(self.plan_price_usd).as_deref())?;
        Ok(())
    }
}

/// Where Claude Code keeps transcripts: `$CLAUDE_CONFIG_DIR/projects`, else
/// `~/.claude/projects` (on Windows, `%USERPROFILE%\.claude\projects`).
pub fn default_folder() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir).join("projects"));
    }
    dirs::home_dir().map(|h| h.join(".claude").join("projects"))
}

pub fn folder(settings: &CcSettings) -> Option<PathBuf> {
    settings.folder_override.as_ref().map(PathBuf::from).or_else(default_folder)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ScanStats {
    pub files: usize,
    pub lines_parsed: u64,
    pub lines_skipped: u64,
    pub events_added: usize,
}

/// Every `.jsonl` under `root`, a few levels deep (subagent transcripts sit
/// one level below the session's own file), changed within the kept window.
fn transcript_files(root: &Path) -> Vec<PathBuf> {
    let cutoff = SystemTime::now() - Duration::from_secs((KEEP_DAYS as u64) * 86_400);
    let mut out = vec![];
    let mut stack = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                if depth < 3 {
                    stack.push((path, depth + 1));
                }
            } else if path.extension().is_some_and(|e| e == "jsonl")
                && meta.modified().map(|m| m >= cutoff).unwrap_or(true)
            {
                out.push(path);
            }
        }
    }
    out
}

/// Reads whatever was appended to each transcript since the last scan.
/// Only complete lines are consumed; a half-written last line waits for the
/// next scan.
pub fn scan(store: &mut Store, root: &Path) -> std::io::Result<ScanStats> {
    let mut stats = ScanStats::default();
    for path in transcript_files(root) {
        stats.files += 1;
        let key = path.to_string_lossy().to_string();
        let mut offset = store.offset(&key).unwrap_or(0);
        let Ok(mut file) = File::open(&path) else { continue };
        let len = file.metadata()?.len();
        if len < offset {
            // Truncated or replaced: start again (duplicates are ignored).
            offset = 0;
        }
        if len == offset {
            continue;
        }
        file.seek(SeekFrom::Start(offset))?;
        let mut buf = Vec::with_capacity((len - offset) as usize);
        file.take(len - offset).read_to_end(&mut buf)?;
        let Some(last_nl) = buf.iter().rposition(|&b| b == b'\n') else { continue };
        let complete = &buf[..=last_nl];

        let mut events = vec![];
        for line in complete.split(|&b| b == b'\n') {
            if line.is_empty() {
                continue;
            }
            match claude_code::parse_line(&String::from_utf8_lossy(line)) {
                Line::Usage(e) => {
                    stats.lines_parsed += 1;
                    events.push(e);
                }
                Line::Other => stats.lines_parsed += 1,
                Line::Skipped => stats.lines_skipped += 1,
            }
        }
        stats.events_added += store.insert_events(&events).map_err(std::io::Error::other)?;
        store
            .set_offset(&key, offset + complete.len() as u64)
            .map_err(std::io::Error::other)?;
    }
    Ok(stats)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub tokens: i64,
    pub cost_usd: f64,
    pub messages: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CcSummary {
    pub folder: Option<String>,
    pub folder_exists: bool,
    pub files: usize,
    /// Lines read since the app started.
    pub lines_parsed: u64,
    pub lines_skipped: u64,
    pub last_scan_ms: i64,
    pub error: Option<String>,
    /// The last 30 days, per hour, model and project.
    pub hourly: Vec<HourRow>,
    /// The block in progress, if any.
    pub current_block: Option<Block>,
    /// The most recent finished block, if no block is in progress.
    pub last_block: Option<Block>,
    pub week: Totals,
    pub unpriced_models: Vec<String>,
    pub settings: CcSettings,
}

pub struct Health {
    pub folder: Option<PathBuf>,
    pub files: usize,
    pub lines_parsed: u64,
    pub lines_skipped: u64,
    pub last_scan_ms: i64,
    pub error: Option<String>,
}

pub fn summary(store: &Store, now_ms: i64, health: &Health, settings: CcSettings) -> CcSummary {
    let hourly = store.hourly(now_ms - 30 * DAY_MS).unwrap_or_default();
    // A day of history is enough to place the current block.
    let timeline = store.timeline(now_ms - DAY_MS).unwrap_or_default();
    let blocks = claude_code::blocks(&timeline);
    let (current_block, last_block) = match blocks.last() {
        Some(b) if b.end_ms > now_ms => (Some(b.clone()), None),
        Some(b) => (None, Some(b.clone())),
        None => (None, None),
    };
    let week_from = now_ms - 7 * DAY_MS;
    let week = hourly.iter().filter(|r| r.hour_ms >= week_from).fold(
        Totals { tokens: 0, cost_usd: 0.0, messages: 0 },
        |mut t, r| {
            t.tokens += r.tokens;
            t.cost_usd += r.cost_usd;
            t.messages += r.messages;
            t
        },
    );
    CcSummary {
        folder: health.folder.as_ref().map(|p| p.to_string_lossy().to_string()),
        folder_exists: health.folder.as_ref().is_some_and(|p| p.is_dir()),
        files: health.files,
        lines_parsed: health.lines_parsed,
        lines_skipped: health.lines_skipped,
        last_scan_ms: health.last_scan_ms,
        error: health.error.clone(),
        hourly,
        current_block,
        last_block,
        week,
        unpriced_models: store.unpriced_models(now_ms - 30 * DAY_MS).unwrap_or_default(),
        settings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("co-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("-home-mark-src-claude-overview")).unwrap();
        d
    }

    #[test]
    fn scan_reads_only_new_complete_lines() {
        let root = tmpdir("scan");
        let file = root.join("-home-mark-src-claude-overview").join("s1.jsonl");
        let sample = include_str!("../tests/fixtures/claude-code-sample.jsonl");
        let lines: Vec<&str> = sample.lines().collect();
        let mut store = Store::open_in_memory().unwrap();

        // First three lines, plus half of the fourth.
        let mut f = File::create(&file).unwrap();
        writeln!(f, "{}\n{}\n{}", lines[0], lines[1], lines[2]).unwrap();
        write!(f, "{}", &lines[4][..20]).unwrap();
        drop(f);
        let s1 = scan(&mut store, &root).unwrap();
        assert_eq!(s1.files, 1);
        assert_eq!(s1.events_added, 1, "msg_A once, despite two lines");

        // Finish the half line and add the rest.
        let mut f = std::fs::OpenOptions::new().append(true).open(&file).unwrap();
        writeln!(f, "{}", &lines[4][20..]).unwrap();
        for l in &lines[5..] {
            writeln!(f, "{l}").unwrap();
        }
        drop(f);
        let s2 = scan(&mut store, &root).unwrap();
        assert_eq!(s2.events_added, 3, "msg_B, msg_C, msg_D");
        assert_eq!(s2.lines_skipped, 0);

        // Nothing new: nothing read.
        let s3 = scan(&mut store, &root).unwrap();
        assert_eq!(s3.lines_parsed + s3.lines_skipped, 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn summary_finds_the_block_in_progress() {
        let root = tmpdir("summary");
        std::fs::write(
            root.join("-home-mark-src-claude-overview").join("s.jsonl"),
            include_str!("../tests/fixtures/claude-code-sample.jsonl"),
        )
        .unwrap();
        let mut store = Store::open_in_memory().unwrap();
        let stats = scan(&mut store, &root).unwrap();
        assert_eq!(stats.lines_skipped, 1);
        let now = claude_code::parse_rfc3339_ms("2026-11-23T12:00:00Z").unwrap();
        let health = Health {
            folder: Some(root.clone()),
            files: 1,
            lines_parsed: stats.lines_parsed,
            lines_skipped: stats.lines_skipped,
            last_scan_ms: now,
            error: None,
        };
        let s = summary(&store, now, &health, CcSettings::default());
        let b = s.current_block.expect("block in progress");
        assert_eq!(b.start_ms, claude_code::parse_rfc3339_ms("2026-11-23T09:00:00Z").unwrap());
        assert_eq!(b.messages, 3);
        assert_eq!(s.week.messages, 4);
        assert!(s.folder_exists);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn settings_round_trip_and_drop_bad_numbers() {
        let store = Store::open_in_memory().unwrap();
        let s = CcSettings {
            folder_override: Some("  ".into()),
            block_limit_usd: Some(40.0),
            week_limit_usd: Some(-1.0),
            plan_price_usd: Some(f64::NAN),
        };
        s.save(&store).unwrap();
        let back = CcSettings::load(&store);
        assert_eq!(back.folder_override, None);
        assert_eq!(back.block_limit_usd, Some(40.0));
        assert_eq!(back.week_limit_usd, None);
        assert_eq!(back.plan_price_usd, None);
    }
}
