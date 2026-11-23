import { describe, expect, it } from "vitest";
import { byModel, creditDays, daysUntil } from "./credit";

const D = 86_400_000;
const start = Date.UTC(2026, 10, 17);
const end = Date.UTC(2026, 11, 17);

describe("credit helpers", () => {
  it("lists each UTC day of the period up to today", () => {
    const now = start + 6 * D + 5 * 3_600_000;
    const days = creditDays(
      [
        { dayMs: start, model: "claude-opus-5-5", usd: 4 },
        { dayMs: start, model: "claude-sonnet-5-5", usd: 1 },
        { dayMs: start + 6 * D, model: "claude-opus-5-5", usd: 2 },
        { dayMs: start - D, model: "claude-opus-5-5", usd: 99 },
      ],
      start,
      end,
      now,
    );
    expect(days).toHaveLength(7);
    expect(days[0].key).toBe("2026-11-17");
    expect(days[0].byFamily).toEqual({ opus: 4, sonnet: 1 });
    expect(days.map((d) => d.costUsd)).toEqual([5, 0, 0, 0, 0, 0, 2]);
  });

  it("stops at the period end", () => {
    expect(creditDays([], start, end, end + 3 * D)).toHaveLength(30);
  });

  it("totals by model and counts days left", () => {
    expect(
      byModel([
        { dayMs: 0, model: "a", usd: 1 },
        { dayMs: 0, model: "b", usd: 3 },
        { dayMs: D, model: "a", usd: 1 },
      ]),
    ).toEqual([
      { model: "b", usd: 3 },
      { model: "a", usd: 2 },
    ]);
    expect(daysUntil(end, end - 23 * D - 1)).toBe(24);
    expect(daysUntil(end, end + 1)).toBe(0);
  });
});
