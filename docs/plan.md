# Plan

The living plan is a Claude Doc: <https://claude.ai/artifact/BegSD6Z9Rup6s4Qu1yzDVc>. This file records the decisions so the repo stands on its own.

## Decisions (23 November 2026)

| # | Decision | Answer |
| --- | --- | --- |
| 1 | Accounts | Max subscription plus the Console org holding its $100 monthly API credit (resets on the 17th) |
| 2 | Max plan limits | Local Claude Code logs measured against limits entered in Settings, plus a link to claude.ai usage. No unofficial endpoints. |
| 3 | What's waiting | Claude Code sessions waiting for input, Message Batches in progress, GitHub PRs by Claude or awaiting review |
| 4 | Shape | Tray icon, pop-over and main window |
| 5 | Refresh | Status 2 min, local logs and PRs 5 min, API usage 30 min |
| 6 | Alerts | Outage, 80% of plan or credit, new PR to review |
| 7 | Stack | Tauri 2, React + TypeScript |
| 8 | Shipping | GitHub Actions on tag for Windows, macOS and Linux; unsigned; Tauri signed updater |
| 9 | Platforms in daily use | Windows and Linux (macOS is built but not tested on hardware) |

## What the APIs can and can't give

| Need | Source | Credential |
| --- | --- | --- |
| Service status | `status.claude.com/api/v2/*.json` | None |
| API spend and tokens | Admin API usage and cost analytics (beta), 4 to 24 h lag; legacy usage report about 30 min lag | Admin API key |
| Batches, models | Messages API | Standard API key |
| Max plan usage | `~/.claude/projects/*/*.jsonl` (undocumented format) | None |
| PRs | GitHub search | Read-only fine-grained token |

There is no public API for Max plan percentages, claude.ai Projects or claude.ai chats.

## Milestones

1. Skeleton and status: tray, pop-over, main window, status panel, CI. **Done.**
2. Claude Code usage: JSONL reader, SQLite store, per-project usage, 5-hour block countdown. **(this PR)**
3. API credit: keychain, Admin API, spend vs allowance, burn rate and forecast.
4. What's waiting: Claude Code sessions, batches, PRs.
5. Alerts, global hotkey, release workflow and updater.
