import type { CostRow } from "../types";
import type { Day } from "./usage";
import { DAY_MS, modelFamily } from "./usage";

/** One entry per UTC day from the period start to today, with spend by model family. */
export function creditDays(rows: CostRow[], startMs: number, endMs: number, nowMs: number = Date.now()): Day[] {
  const last = Math.min(Math.floor(nowMs / DAY_MS) * DAY_MS, endMs - DAY_MS);
  const days: Day[] = [];
  const index = new Map<number, Day>();
  for (let t = startMs; t <= last; t += DAY_MS) {
    const d: Day = { key: new Date(t).toISOString().slice(0, 10), startMs: t, costUsd: 0, tokens: 0, byFamily: {} };
    days.push(d);
    index.set(t, d);
  }
  for (const r of rows) {
    const d = index.get(Math.floor(r.dayMs / DAY_MS) * DAY_MS);
    if (!d) continue;
    d.costUsd += r.usd;
    const f = modelFamily(r.model);
    d.byFamily[f] = (d.byFamily[f] ?? 0) + r.usd;
  }
  return days;
}

/** Spend per model this period, largest first. */
export function byModel(rows: CostRow[]): { model: string; usd: number }[] {
  const m = new Map<string, number>();
  for (const r of rows) m.set(r.model, (m.get(r.model) ?? 0) + r.usd);
  return [...m.entries()].map(([model, usd]) => ({ model, usd })).sort((a, b) => b.usd - a.usd);
}

/** Whole days until `endMs`, counting a part day as one. */
export function daysUntil(endMs: number, nowMs: number = Date.now()): number {
  return Math.max(0, Math.ceil((endMs - nowMs) / DAY_MS));
}

/** "Thu 17 Dec" in the user's locale, read as a UTC date. */
export function utcDate(ms: number): string {
  return new Date(ms).toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short", timeZone: "UTC" });
}
