import type { Batch, Marker, WaitingSnapshot } from "../types";
import { formatCount } from "./format";

/** Plain words for why a session is waiting. */
export function markerLabel(kind: Marker["kind"]): string {
  switch (kind) {
    case "permission":
      return "Needs permission";
    case "question":
      return "Has a question";
    case "finished":
      return "Finished, your turn";
    default:
      return "Waiting for you";
  }
}

/** One word for the tight lists in the Overview card and the pop-over. */
export function markerShort(kind: Marker["kind"]): string {
  switch (kind) {
    case "permission":
      return "Permission";
    case "question":
      return "Question";
    case "finished":
      return "Your turn";
    default:
      return "Idle";
  }
}

export function isRunning(b: Batch): boolean {
  return b.status !== "ended";
}

/** "40 of 100 done" for a running batch; "50 done, 2 failed" for an ended one. */
export function batchProgress(b: Batch): string {
  const c = b.counts;
  const total = c.processing + c.succeeded + c.errored + c.canceled + c.expired;
  const failed = c.errored + c.expired;
  const n = formatCount;
  if (isRunning(b)) return `${n(total - c.processing)} of ${n(total)} done`;
  return failed > 0 ? `${n(c.succeeded)} done, ${n(failed)} failed` : `${n(c.succeeded)} done`;
}

export interface WaitingCounts {
  sessions: number;
  batches: number;
  reviews: number;
  claude: number;
  /** Things that need you: sessions, reviews and Claude's PRs. */
  total: number;
}

/** Sessions only count while the hooks are in place; older markers may be stale. */
export function liveSessions(w: WaitingSnapshot | null): WaitingSnapshot["sessions"] {
  return w?.hooks.installed ? w.sessions : [];
}

/** True once any of the three sources is set up. */
export function anythingSetUp(w: WaitingSnapshot): boolean {
  return w.hooks.installed || w.batches.configured || w.prs.configured;
}

export function waitingCounts(w: WaitingSnapshot | null): WaitingCounts {
  const prs = w?.prs.items ?? [];
  const sessions = liveSessions(w).length;
  const reviews = prs.filter((p) => p.reason === "review").length;
  const claude = prs.filter((p) => p.reason === "claude").length;
  return {
    sessions,
    batches: (w?.batches.items ?? []).filter(isRunning).length,
    reviews,
    claude,
    total: sessions + reviews + claude,
  };
}
