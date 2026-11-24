import type { WaitingSnapshot } from "../types";
import { relativeTime } from "../lib/format";
import { anythingSetUp, batchProgress, isRunning, liveSessions, markerShort, waitingCounts } from "../lib/waiting";
import { Chip, markerChip } from "./Chip";
import { Link } from "./Link";

/** A short list of what needs you, for the Overview card and the pop-over. */
export function WaitingList({ waiting, now, onSetUp }: { waiting: WaitingSnapshot | null; now: number; onSetUp?: () => void }) {
  if (!waiting || !waiting.checked) return <p className="meta">Checking…</p>;
  const c = waitingCounts(waiting);
  if (!anythingSetUp(waiting)) {
    return (
      <div className="prompt">
        <p className="meta">Set up Claude Code hooks, an API key or a GitHub token to see what's waiting.</p>
        {onSetUp && (
          <button type="button" onClick={onSetUp}>
            Set up
          </button>
        )}
      </div>
    );
  }
  const prs = waiting.prs.items ?? [];
  const running = (waiting.batches.items ?? []).filter(isRunning);
  if (c.total === 0 && running.length === 0) return <p className="meta">Nothing waiting for you.</p>;

  return (
    <ul className="waiting">
      {liveSessions(waiting).map((s) => {
        const chip = markerChip(s.kind);
        return (
          <li key={s.sessionId}>
            <span className="chip-cell">
              <Chip tone={chip.tone} symbol={chip.symbol}>
                {markerShort(s.kind)}
              </Chip>
            </span>
            <span className="name">{s.project || "Claude Code"}</span>
            <span className="meta end">{relativeTime(s.atMs, now)}</span>
          </li>
        );
      })}
      {prs.slice(0, 5).map((p) => (
        <li key={p.url}>
          <span className="chip-cell">
            <Chip>{p.reason === "review" ? "Review" : "Claude"}</Chip>
          </span>
          <Link href={p.url} className="name">
            {p.title}
          </Link>
          <span className="meta end">#{p.number}</span>
        </li>
      ))}
      {prs.length > 5 && <li className="meta more">and {prs.length - 5} more pull requests</li>}
      {running.map((b) => (
        <li key={b.id}>
          <span className="chip-cell">
            <Chip>Batch</Chip>
          </span>
          <span className="name">{batchProgress(b)}</span>
          <span className="meta end">{relativeTime(b.createdMs, now)}</span>
        </li>
      ))}
    </ul>
  );
}
