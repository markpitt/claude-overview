import { describe, expect, it } from "vitest";
import type { HourRow } from "../types";
import { byLocalDay, byProject, countdown, formatTokens, formatUsd, modelFamily, share, sumCost } from "./usage";

const H = 3_600_000;

function row(hourMs: number, model: string, project: string, costUsd: number, tokens = 100): HourRow {
  return { hourMs, model, project, tokens, output: 10, costUsd, messages: 1, sessions: 1 };
}

describe("usage helpers", () => {
  it("groups by local day, oldest first, with gaps filled", () => {
    const now = new Date(2026, 10, 23, 15, 0).getTime();
    const today = new Date(2026, 10, 23, 9, 0).getTime();
    const twoAgo = new Date(2026, 10, 21, 22, 0).getTime();
    const days = byLocalDay(
      [
        row(today, "claude-opus-5-5", "a", 2),
        row(today + H, "claude-sonnet-5-5", "a", 1),
        row(twoAgo, "claude-opus-5-5", "b", 4),
      ],
      3,
      now,
    );
    expect(days.map((d) => d.costUsd)).toEqual([4, 0, 3]);
    expect(days[2].byFamily).toEqual({ opus: 2, sonnet: 1 });
  });

  it("ignores rows outside the window", () => {
    const now = new Date(2026, 10, 23, 15, 0).getTime();
    const days = byLocalDay([row(now - 40 * 24 * H, "claude-opus-5-5", "a", 9)], 30, now);
    expect(days.reduce((t, d) => t + d.costUsd, 0)).toBe(0);
  });

  it("ranks projects by cost and keeps the latest activity", () => {
    const p = byProject(
      [row(1 * H, "m", "a", 1), row(3 * H, "m", "a", 1), row(2 * H, "m", "b", 5), row(0, "m", "old", 99)],
      H,
    );
    expect(p.map((x) => x.project)).toEqual(["b", "a"]);
    expect(p[1].lastMs).toBe(3 * H);
    expect(p[1].messages).toBe(2);
    expect(sumCost([row(0, "m", "a", 1), row(H, "m", "a", 2)], H)).toBe(2);
  });

  it("names model families", () => {
    expect(modelFamily("claude-opus-5-5")).toBe("opus");
    expect(modelFamily("claude-fable-5-2")).toBe("fable");
    expect(modelFamily("gpt-x")).toBe("other");
  });

  it("formats money, tokens and countdowns", () => {
    expect(formatUsd(0.4)).toBe("$0.40");
    expect(formatUsd(1204.4)).toBe("$1,204");
    expect(formatTokens(950)).toBe("950");
    expect(formatTokens(12_340)).toBe("12.3k");
    expect(formatTokens(29_366_558)).toBe("29.4M");
    expect(formatTokens(250_000_000)).toBe("250M");
    expect(countdown(10 * 60000 + 1, 0)).toBe("11 min");
    expect(countdown(134 * 60000, 0)).toBe("2 h 14 min");
    expect(countdown(-5, 0)).toBe("0 min");
  });

  it("works out the share of a limit", () => {
    expect(share(5, null)).toBeNull();
    expect(share(5, 20)).toBe(0.25);
    expect(share(50, 20)).toBe(1);
  });
});
