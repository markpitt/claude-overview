import { describe, expect, it } from "vitest";
import type { Batch, WaitingSnapshot } from "../types";
import { batchProgress, markerLabel, markerShort, waitingCounts } from "./waiting";

function batch(status: string, processing: number, succeeded: number, errored = 0): Batch {
  return {
    id: "b",
    status,
    counts: { processing, succeeded, errored, canceled: 0, expired: 0 },
    createdMs: 0,
    endedMs: null,
    expiresMs: null,
  };
}

describe("waiting helpers", () => {
  it("describes batches", () => {
    expect(batchProgress(batch("in_progress", 60, 40))).toBe("40 of 100 done");
    expect(batchProgress(batch("in_progress", 640, 1360))).toBe("1,360 of 2,000 done");
    expect(batchProgress(batch("ended", 0, 50, 2))).toBe("50 done, 2 failed");
    expect(batchProgress(batch("ended", 0, 5))).toBe("5 done");
  });

  it("labels sessions", () => {
    expect(markerLabel("permission")).toBe("Needs permission");
    expect(markerLabel("finished")).toBe("Finished, your turn");
    expect(markerShort("idle")).toBe("Idle");
  });

  it("counts what needs you", () => {
    const pr = (reason: "review" | "claude") => ({
      repo: "a/b",
      number: 1,
      title: "t",
      url: `u-${Math.random()}`,
      author: "m",
      draft: false,
      updatedMs: 0,
      reason,
    });
    const w: WaitingSnapshot = {
      checked: true,
      sessions: [
        { sessionId: "s", project: "p", cwd: null, kind: "idle", message: null, transcriptPath: null, atMs: 0 },
      ],
      hooks: { installed: true, settingsPath: null, error: null },
      batches: { configured: true, keyHint: null, error: null, fetchedAtMs: 0, items: [batch("in_progress", 1, 0), batch("ended", 0, 1)] },
      prs: { configured: true, keyHint: null, error: null, fetchedAtMs: 0, items: [pr("review"), pr("claude"), pr("claude")] },
    };
    expect(waitingCounts(w)).toEqual({ sessions: 1, batches: 1, reviews: 1, claude: 2, total: 4 });
    expect(waitingCounts(null).total).toBe(0);
    expect(waitingCounts({ ...w, hooks: { ...w.hooks, installed: false } }).sessions).toBe(0);
  });
});
