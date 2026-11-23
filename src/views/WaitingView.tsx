import { useApp } from "../store";
import { useNow } from "../lib/useNow";
import { relativeTime } from "../lib/format";
import { batchProgress, isRunning, markerLabel } from "../lib/waiting";
import { Link } from "../components/Link";
import type { Source } from "../types";

function SourceNote<T>({ src, now, what }: { src: Source<T>; now: number; what: string }) {
  return (
    <>
      {src.error && (
        <p className="error small">
          {src.error}
          {src.items ? " Showing the last list." : ""}
        </p>
      )}
      {src.fetchedAtMs > 0 && (
        <p className="muted small note">
          {what} checked {relativeTime(src.fetchedAtMs, now)}, every 5 minutes.
        </p>
      )}
    </>
  );
}

export function WaitingView({ onShowSettings }: { onShowSettings: () => void }) {
  const w = useApp((s) => s.waiting);
  const busy = useApp((s) => s.waitingBusy);
  const refresh = useApp((s) => s.refreshWaiting);
  const setHooks = useApp((s) => s.setHooks);
  const now = useNow(15_000);

  if (!w || !w.checked) return <p className="muted">Checking…</p>;
  const prs = w.prs.items ?? [];
  const reviews = prs.filter((p) => p.reason === "review");
  const claude = prs.filter((p) => p.reason === "claude");
  const batches = w.batches.items ?? [];

  return (
    <div className="stack">
      <section className="card">
        <div className="row">
          <h2 className="flush">Claude Code sessions</h2>
          <span className="spacer" />
          <button type="button" className="ghost" disabled={busy} onClick={() => void refresh()}>
            {busy ? "Checking…" : "Refresh all"}
          </button>
        </div>
        {!w.hooks.installed ? (
          <div className="gap-top small">
            <p>
              Claude Code tells this app when a session needs you through hooks: it runs Claude Overview when it asks for
              permission, when it's idle, when it finishes a turn and when a session ends.
            </p>
            <p className="muted">
              This adds three entries to <code>{w.hooks.settingsPath ?? "~/.claude/settings.json"}</code> and keeps a
              backup of the file beside it. Your other settings and hooks are left as they are.
            </p>
            <button type="button" disabled={busy} onClick={() => void setHooks(true).catch(() => {})}>
              Add the hooks
            </button>
            {w.hooks.error && <p className="error small">{w.hooks.error}</p>}
          </div>
        ) : w.sessions.length === 0 ? (
          <p className="muted gap-top">No sessions waiting for you.</p>
        ) : (
          <ul className="waiting gap-top">
            {w.sessions.map((s) => (
              <li key={s.sessionId}>
                <span className={`tag tag-${s.kind}`}>{markerLabel(s.kind)}</span>
                <div className="name">
                  <div>{s.project || "Claude Code"}</div>
                  {s.message && <div className="muted small clip">{s.message}</div>}
                  {s.cwd && <code className="muted small">{s.cwd}</code>}
                </div>
                <span className="muted small">{relativeTime(s.atMs, now)}</span>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="card">
        <h2>Pull requests</h2>
        {!w.prs.configured ? (
          <p className="muted small">
            Add a GitHub token in <a onClick={onShowSettings}>Settings</a> to see pull requests waiting for your review
            and those Claude opened.
          </p>
        ) : (
          <>
            <h3>Waiting for your review</h3>
            {reviews.length === 0 ? <p className="muted small">None.</p> : <PrList prs={reviews} now={now} />}
            <h3>From Claude</h3>
            {claude.length === 0 ? <p className="muted small">None open.</p> : <PrList prs={claude} now={now} />}
            <SourceNote src={w.prs} now={now} what="GitHub" />
          </>
        )}
      </section>

      <section className="card">
        <h2>Message batches</h2>
        {!w.batches.configured ? (
          <p className="muted small">
            Add an API key in <a onClick={onShowSettings}>Settings</a> to see batches in progress.
          </p>
        ) : (
          <>
            {batches.length === 0 ? (
              <p className="muted small">No batches running or finished in the last day.</p>
            ) : (
              <table className="table">
                <thead>
                  <tr>
                    <th>Batch</th>
                    <th>State</th>
                    <th className="num">Progress</th>
                    <th className="num">Started</th>
                  </tr>
                </thead>
                <tbody>
                  {batches.map((b) => (
                    <tr key={b.id}>
                      <td>
                        <code>{b.id}</code>
                      </td>
                      <td className={isRunning(b) ? "" : "muted"}>
                        {b.status === "in_progress" ? "Running" : b.status === "canceling" ? "Cancelling" : "Ended"}
                      </td>
                      <td className="num">{batchProgress(b)}</td>
                      <td className="num muted">{relativeTime(b.createdMs, now)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            <SourceNote src={w.batches} now={now} what="Batches" />
          </>
        )}
      </section>
    </div>
  );
}

function PrList({ prs, now }: { prs: { url: string; repo: string; number: number; title: string; draft: boolean; updatedMs: number }[]; now: number }) {
  return (
    <ul className="waiting">
      {prs.map((p) => (
        <li key={p.url}>
          <span className="muted small repo">
            {p.repo}#{p.number}
          </span>
          <Link href={p.url} className="name">
            {p.title}
          </Link>
          {p.draft && <span className="badge">Draft</span>}
          <span className="muted small">{relativeTime(p.updatedMs, now)}</span>
        </li>
      ))}
    </ul>
  );
}
