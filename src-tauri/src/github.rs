//! Pull requests waiting on you, from GitHub search: open PRs where your
//! review is requested, and open PRs of yours that Claude wrote (their
//! description carries the Claude Code footer). Needs a read-only token.

use serde::{Deserialize, Serialize};

use crate::claude_code::parse_rfc3339_ms;

pub const GITHUB_API: &str = "https://api.github.com";

/// GitHub API base; `CLAUDE_OVERVIEW_GITHUB_BASE` points it at a test server.
fn base() -> String {
    std::env::var("CLAUDE_OVERVIEW_GITHUB_BASE")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| GITHUB_API.to_string())
}

#[derive(Debug, Deserialize)]
struct RawSearch {
    #[serde(default)]
    items: Vec<RawItem>,
}

#[derive(Debug, Deserialize)]
struct RawItem {
    number: i64,
    title: String,
    html_url: String,
    repository_url: String,
    updated_at: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    body: Option<String>,
    user: RawUser,
}

#[derive(Debug, Deserialize)]
struct RawUser {
    login: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pr {
    pub repo: String,
    pub number: i64,
    pub title: String,
    pub url: String,
    pub author: String,
    pub draft: bool,
    pub updated_ms: i64,
    /// "review" (your review is requested) or "claude" (Claude wrote it).
    pub reason: String,
}

/// True when a PR description carries Claude Code's footer.
fn by_claude(body: &str) -> bool {
    body.contains("Generated with [Claude Code]") || body.contains("claude.ai/code/session_")
}

fn parse(body: &str, reason: &str) -> Result<Vec<Pr>, String> {
    let raw: RawSearch = serde_json::from_str(body).map_err(|e| format!("unexpected GitHub response: {e}"))?;
    Ok(raw
        .items
        .into_iter()
        .filter(|i| reason != "claude" || i.body.as_deref().is_some_and(by_claude))
        .map(|i| Pr {
            repo: i.repository_url.rsplitn(3, '/').take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("/"),
            number: i.number,
            title: i.title,
            url: i.html_url,
            author: i.user.login,
            draft: i.draft,
            updated_ms: parse_rfc3339_ms(&i.updated_at).unwrap_or(0),
            reason: reason.into(),
        })
        .collect())
}

async fn search(client: &reqwest::Client, token: &str, q: &str, reason: &str) -> Result<Vec<Pr>, String> {
    let resp = client
        .get(format!("{}/search/issues", base()))
        .query(&[("q", q), ("sort", "updated"), ("per_page", "50")])
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| format!("Couldn't reach GitHub: {}.", e.without_url()))?;
    let status = resp.status();
    let body = resp.text().await.map_err(|e| format!("Reading GitHub's response: {}.", e.without_url()))?;
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 => "GitHub rejected the token.".into(),
            403 | 429 => "GitHub rate limit reached; will retry.".into(),
            s => format!("GitHub error {s}."),
        });
    }
    parse(&body, reason)
}

/// PRs needing your review first, then Claude's; a PR in both counts once.
pub async fn fetch(client: &reqwest::Client, token: &str) -> Result<Vec<Pr>, String> {
    let mut prs = search(client, token, "is:pr is:open archived:false review-requested:@me", "review").await?;
    let claude = search(client, token, "is:pr is:open archived:false author:@me", "claude").await?;
    for p in claude {
        if !prs.iter().any(|q| q.url == p.url) {
            prs.push(p);
        }
    }
    Ok(prs)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"{"total_count":2,"incomplete_results":false,"items":[
      {"number":4,"title":"Milestone 4: What's waiting","html_url":"https://github.com/markpitt/claude-overview/pull/4",
       "repository_url":"https://api.github.com/repos/markpitt/claude-overview","updated_at":"2026-11-23T21:00:00Z",
       "draft":false,"user":{"login":"markpitt"},"pull_request":{},
       "body":"Adds things.\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)"},
      {"number":9,"title":"Hand-written change","html_url":"https://github.com/markpitt/other/pull/9",
       "repository_url":"https://api.github.com/repos/markpitt/other","updated_at":"2026-11-22T09:00:00Z",
       "draft":true,"user":{"login":"markpitt"},"pull_request":{},"body":null}
    ]}"#;

    #[test]
    fn keeps_only_claudes_prs_for_the_claude_search() {
        let c = parse(BODY, "claude").unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].repo.as_str(), c[0].number), ("markpitt/claude-overview", 4));
        let r = parse(BODY, "review").unwrap();
        assert_eq!(r.len(), 2);
        assert!(r[1].draft);
        assert!(parse("{\"message\":\"Bad credentials\"", "review").is_err());
    }
}
