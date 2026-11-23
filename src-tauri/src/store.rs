//! SQLite cache in the app data folder. Holds de-duplicated Claude Code
//! usage, how far each transcript has been read, and plain settings.
//! No secrets are ever stored here; keys go in the OS keychain.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::claude_code::{UsageEvent, api_cost_usd};

/// History kept, per the plan.
pub const KEEP_DAYS: i64 = 90;
const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;

pub struct Store {
    conn: Connection,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HourRow {
    pub hour_ms: i64,
    pub model: String,
    pub project: String,
    pub tokens: i64,
    pub output: i64,
    pub cost_usd: f64,
    pub messages: i64,
    pub sessions: i64,
}

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS usage_events (
                 message_id TEXT NOT NULL,
                 request_id TEXT NOT NULL,
                 ts_ms INTEGER NOT NULL,
                 model TEXT NOT NULL,
                 project TEXT NOT NULL,
                 session_id TEXT NOT NULL,
                 input INTEGER NOT NULL,
                 output INTEGER NOT NULL,
                 cache_read INTEGER NOT NULL,
                 cache_write_5m INTEGER NOT NULL,
                 cache_write_1h INTEGER NOT NULL,
                 cost_usd REAL,
                 PRIMARY KEY (message_id, request_id)
             );
             CREATE INDEX IF NOT EXISTS usage_events_ts ON usage_events(ts_ms);
             CREATE TABLE IF NOT EXISTS file_offsets (
                 path TEXT PRIMARY KEY,
                 offset INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS settings (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );",
        )?;
        Ok(Store { conn })
    }

    /// Inserts events, ignoring any already seen. Returns how many were new.
    pub fn insert_events(&mut self, events: &[UsageEvent]) -> rusqlite::Result<usize> {
        let tx = self.conn.transaction()?;
        let mut added = 0;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR IGNORE INTO usage_events
                 (message_id, request_id, ts_ms, model, project, session_id,
                  input, output, cache_read, cache_write_5m, cache_write_1h, cost_usd)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )?;
            for e in events {
                added += stmt.execute(params![
                    e.message_id,
                    e.request_id,
                    e.ts_ms,
                    e.model,
                    e.project,
                    e.session_id,
                    e.input,
                    e.output,
                    e.cache_read,
                    e.cache_write_5m,
                    e.cache_write_1h,
                    api_cost_usd(e),
                ])?;
            }
        }
        tx.commit()?;
        Ok(added)
    }

    pub fn offset(&self, path: &str) -> rusqlite::Result<u64> {
        self.conn
            .query_row("SELECT offset FROM file_offsets WHERE path = ?1", [path], |r| {
                r.get::<_, i64>(0)
            })
            .optional()
            .map(|o| o.unwrap_or(0).max(0) as u64)
    }

    pub fn set_offset(&self, path: &str, offset: u64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO file_offsets(path, offset) VALUES (?1, ?2)
             ON CONFLICT(path) DO UPDATE SET offset = excluded.offset",
            params![path, offset as i64],
        )?;
        Ok(())
    }

    /// Forgets read positions, so the next scan reads every file again.
    /// Used when the Claude folder changes; duplicates are still ignored.
    pub fn reset_offsets(&self) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM file_offsets", [])?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
            .optional()
    }

    pub fn set_setting(&self, key: &str, value: Option<&str>) -> rusqlite::Result<()> {
        match value {
            Some(v) => self.conn.execute(
                "INSERT INTO settings(key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, v],
            )?,
            None => self.conn.execute("DELETE FROM settings WHERE key = ?1", [key])?,
        };
        Ok(())
    }

    /// Usage per hour, model and project since `from_ms`. The UI buckets
    /// these into local days, so time zones stay right.
    pub fn hourly(&self, from_ms: i64) -> rusqlite::Result<Vec<HourRow>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT (ts_ms / ?2) * ?2 AS hour, model, project,
                    SUM(input + output + cache_read + cache_write_5m + cache_write_1h),
                    SUM(output), COALESCE(SUM(cost_usd), 0), COUNT(*),
                    COUNT(DISTINCT session_id)
             FROM usage_events WHERE ts_ms >= ?1
             GROUP BY hour, model, project ORDER BY hour",
        )?;
        let rows = stmt.query_map(params![from_ms, HOUR_MS], |r| {
            Ok(HourRow {
                hour_ms: r.get(0)?,
                model: r.get(1)?,
                project: r.get(2)?,
                tokens: r.get(3)?,
                output: r.get(4)?,
                cost_usd: r.get(5)?,
                messages: r.get(6)?,
                sessions: r.get(7)?,
            })
        })?;
        rows.collect()
    }

    /// (timestamp, tokens, cost) per response since `from_ms`, oldest first,
    /// for working out 5-hour blocks.
    pub fn timeline(&self, from_ms: i64) -> rusqlite::Result<Vec<(i64, i64, f64)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT ts_ms, input + output + cache_read + cache_write_5m + cache_write_1h,
                    COALESCE(cost_usd, 0)
             FROM usage_events WHERE ts_ms >= ?1 ORDER BY ts_ms",
        )?;
        let rows = stmt.query_map([from_ms], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        rows.collect()
    }

    /// Models seen without a known price, so the UI can say so.
    pub fn unpriced_models(&self, from_ms: i64) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT DISTINCT model FROM usage_events WHERE ts_ms >= ?1 AND cost_usd IS NULL",
        )?;
        let rows = stmt.query_map([from_ms], |r| r.get(0))?;
        rows.collect()
    }

    pub fn prune(&self, now_ms: i64) -> rusqlite::Result<usize> {
        self.conn.execute(
            "DELETE FROM usage_events WHERE ts_ms < ?1",
            [now_ms - KEEP_DAYS * DAY_MS],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: &str, ts_ms: i64, model: &str, output: i64) -> UsageEvent {
        UsageEvent {
            message_id: id.into(),
            request_id: format!("req-{id}"),
            ts_ms,
            model: model.into(),
            project: "proj".into(),
            session_id: "s1".into(),
            input: 10,
            output,
            cache_read: 0,
            cache_write_5m: 0,
            cache_write_1h: 0,
        }
    }

    #[test]
    fn inserts_are_idempotent_and_priced() {
        let mut s = Store::open_in_memory().unwrap();
        let a = ev("a", 1_000, "claude-opus-5-5", 1_000_000);
        assert_eq!(s.insert_events(&[a.clone(), a.clone()]).unwrap(), 1);
        assert_eq!(s.insert_events(&[a]).unwrap(), 0);
        let rows = s.hourly(0).unwrap();
        assert_eq!(rows.len(), 1);
        // 1M output tokens at $20 plus 10 input tokens.
        assert!((rows[0].cost_usd - 20.00004).abs() < 1e-9);
    }

    #[test]
    fn hourly_groups_by_hour_model_and_project() {
        let mut s = Store::open_in_memory().unwrap();
        s.insert_events(&[
            ev("a", 10 * HOUR_MS + 5, "claude-opus-5-5", 1),
            ev("b", 10 * HOUR_MS + 50, "claude-opus-5-5", 1),
            ev("c", 10 * HOUR_MS + 60, "claude-sonnet-5-5", 1),
            ev("d", 11 * HOUR_MS, "claude-opus-5-5", 1),
        ])
        .unwrap();
        let rows = s.hourly(0).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].messages + rows[1].messages, 3);
        assert_eq!(rows[2].hour_ms, 11 * HOUR_MS);
    }

    #[test]
    fn unknown_models_are_stored_without_cost() {
        let mut s = Store::open_in_memory().unwrap();
        s.insert_events(&[ev("x", 5, "claude-future-9", 1)]).unwrap();
        assert_eq!(s.unpriced_models(0).unwrap(), vec!["claude-future-9".to_string()]);
    }

    #[test]
    fn offsets_and_settings_round_trip() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.offset("/a.jsonl").unwrap(), 0);
        s.set_offset("/a.jsonl", 42).unwrap();
        assert_eq!(s.offset("/a.jsonl").unwrap(), 42);
        s.reset_offsets().unwrap();
        assert_eq!(s.offset("/a.jsonl").unwrap(), 0);
        s.set_setting("k", Some("v")).unwrap();
        assert_eq!(s.setting("k").unwrap().as_deref(), Some("v"));
        s.set_setting("k", None).unwrap();
        assert_eq!(s.setting("k").unwrap(), None);
    }

    #[test]
    fn prune_drops_old_rows() {
        let mut s = Store::open_in_memory().unwrap();
        let now = 200 * DAY_MS;
        s.insert_events(&[ev("old", now - 91 * DAY_MS, "claude-opus-5-5", 1), ev("new", now, "claude-opus-5-5", 1)])
            .unwrap();
        assert_eq!(s.prune(now).unwrap(), 1);
    }
}
