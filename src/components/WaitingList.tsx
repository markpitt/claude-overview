import type { WaitingSnapshot } from "../types";
import { relativeTime } from "../lib/format";
import { anythingSetUp, batchProgress, isRunning, liveSessions, markerShort, waitingCounts } from "../lib/waiting";
import { Link } from "./Link";

/** A short list of what needs you, for the Overview card and the pop-over. */
export function WaitingList({ waiting, now, onSetUp }: { waiting: WaitingSnapshot | null; now: number; onSetUp?: () => void }) {
  if (!waiting || !waiting.checked) return <p className="muted small">Checking…</p>;
  const c = waitingCounts(waiting);
  if (!anythingSetUp(waiting)) {
    return (
      <div className="small">
        <p className="muted">Set up Claude Code hooks, an API key or a GitHub token to see what's waiting.</p>
        {onSetUp && (
          <button type="button" className="ghost" onClick={onSetUp}>
            Set up
          </button>
        )}
      </div>
    );
  }
  const prs = waiting.prs.items ?? [];
  const running = (waiting.batches.items ?? []).filter(isRunning);
  if (c.total === 0 && running.length === 0) return <p className="muted small">Nothing waiting for you.</p>;

  return (
    <ul className="waiting compact">
      {liveSessions(waiting).map((s) => (
        <li key={s.sessionId}>
          <span className={`tag tag-${s.kind}`}>{markerShort(s.kind)}</span>
          <span className="name">{s.project || "Claude Code"}</span>
          <span className="muted small">{relativeTime(s.atMs, now)}</span>
        </li>
      ))}
      {prs.slice(0, 5).map((p) => (
        <li key={p.url}>
          <span className={`tag tag-${p.reason}`}>{p.reason === "review" ? "Review" : "Claude"}</span>
          <Link href={p.url} className="name">
            {p.title}
          </Link>
          <span className="muted small">#{p.number}</span>
        </li>
      ))}
      {prs.length > 5 && <li className="muted small">and {prs.length - 5} more pull requests</li>}
      {running.map((b) => (
        <li key={b.id}>
          <span className="tag tag-batch">Batch</span>
          <span className="name">{batchProgress(b)}</span>
          <span className="muted small">{relativeTime(b.createdMs, now)}</span>
        </li>
      ))}
    </ul>
  );
}
