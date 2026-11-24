import { useApp } from "../store";
import { useNow } from "../lib/useNow";
import { relativeTime } from "../lib/format";
import { batchProgress, isRunning, markerLabel } from "../lib/waiting";
import { Link } from "../components/Link";
import { Chip, markerChip } from "../components/Chip";
import { CardHead, TabLink } from "../components/CardHead";
import type { Source } from "../types";

function SourceNote<T>({ src, now, what }: { src: Source<T>; now: number; what: string }) {
  return (
    <>
      {src.error && (
        <p className="error">
          {src.error}
          {src.items ? " Showing the last list." : ""}
        </p>
      )}
      {src.fetchedAtMs > 0 && (
        <p className="meta note">
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

  if (!w || !w.checked) return <p className="meta">Checking…</p>;
  const prs = w.prs.items ?? [];
  const reviews = prs.filter((p) => p.reason === "review");
  const claude = prs.filter((p) => p.reason === "claude");
  const batches = w.batches.items ?? [];

  return (
    <div className="stack">
      <section className="card">
        <CardHead title="Claude Code sessions">
          <button type="button" disabled={busy} onClick={() => void refresh()}>
            {busy ? "Checking…" : "Refresh all"}
          </button>
        </CardHead>
        {!w.hooks.installed ? (
          <div className="prompt">
            <p>
              Claude Code tells this app when a session needs you through hooks: it runs Claude Overview when it asks for
              permission, when it's idle, when it finishes a turn and when a session ends.
            </p>
            <p className="meta">
              This adds three entries to <code>{w.hooks.settingsPath ?? "~/.claude/settings.json"}</code> and keeps a
              backup of the file beside it. Your other settings and hooks are left as they are.
            </p>
            <button type="button" className="primary" disabled={busy} onClick={() => void setHooks(true).catch(() => {})}>
              Add the hooks
            </button>
            {w.hooks.error && <p className="error">{w.hooks.error}</p>}
          </div>
        ) : w.sessions.length === 0 ? (
          <p className="meta">No sessions waiting for you.</p>
        ) : (
          <ul className="waiting wide">
            {w.sessions.map((s) => {
              const chip = markerChip(s.kind);
              return (
                <li key={s.sessionId}>
                  <span className="chip-cell">
                    <Chip tone={chip.tone} symbol={chip.symbol}>
                      {markerLabel(s.kind)}
                    </Chip>
                  </span>
                  <div className="name">
                    <div>{s.project || "Claude Code"}</div>
                    {s.message && <div className="meta clip">{s.message}</div>}
                    {s.cwd && <code className="meta">{s.cwd}</code>}
                  </div>
                  <span className="meta end">{relativeTime(s.atMs, now)}</span>
                </li>
              );
            })}
          </ul>
        )}
      </section>

      <section className="card">
        <CardHead title="Pull requests" />
        {!w.prs.configured ? (
          <p className="meta">
            Add a GitHub token in <TabLink onClick={onShowSettings}>Settings</TabLink> to see pull requests waiting for your review
            and those Claude opened.
          </p>
        ) : (
          <>
            <h3 className="section-label">Waiting for your review</h3>
            {reviews.length === 0 ? <p className="meta">None.</p> : <PrList prs={reviews} now={now} />}
            <h3 className="section-label">From Claude</h3>
            {claude.length === 0 ? <p className="meta">None open.</p> : <PrList prs={claude} now={now} />}
            <SourceNote src={w.prs} now={now} what="GitHub" />
          </>
        )}
      </section>

      <section className="card">
        <CardHead title="Message batches" />
        {!w.batches.configured ? (
          <p className="meta">
            Add an API key in <TabLink onClick={onShowSettings}>Settings</TabLink> to see batches in progress.
          </p>
        ) : (
          <>
            {batches.length === 0 ? (
              <p className="meta">No batches running or finished in the last day.</p>
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
                      <td className="mono">{b.id}</td>
                      <td className={isRunning(b) ? "" : "meta"}>
                        {b.status === "in_progress" ? "Running" : b.status === "canceling" ? "Cancelling" : "Ended"}
                      </td>
                      <td className="num">{batchProgress(b)}</td>
                      <td className="num meta">{relativeTime(b.createdMs, now)}</td>
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
    <ul className="waiting prs">
      {prs.map((p) => (
        <li key={p.url}>
          <span className="meta repo mono">
            {p.repo}#{p.number}
          </span>
          <span className="name">
            <Link href={p.url}>{p.title}</Link>
            {p.draft && (
              <>
                {" "}
                <Chip>Draft</Chip>
              </>
            )}
          </span>
          <span className="meta end">{relativeTime(p.updatedMs, now)}</span>
        </li>
      ))}
    </ul>
  );
}
