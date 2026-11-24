import type { HourRow } from "../types";

export const DAY_MS = 86_400_000;

/** Local calendar day for a timestamp, as YYYY-MM-DD. */
export function dayKey(ms: number): string {
  const d = new Date(ms);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

export const FAMILIES = ["fable", "mythos", "opus", "sonnet", "haiku", "other"] as const;

/** "opus", "sonnet", "haiku", "fable", "mythos" or "other". */
export function modelFamily(model: string): string {
  for (const f of FAMILIES) {
    if (f !== "other" && model.includes(f)) return f;
  }
  return "other";
}

export interface Day {
  key: string;
  startMs: number;
  costUsd: number;
  tokens: number;
  /** API-price dollars per model family. */
  byFamily: Record<string, number>;
}

/** The last `days` local days ending today, oldest first, gaps filled. */
export function byLocalDay(rows: HourRow[], days: number, nowMs: number = Date.now()): Day[] {
  const out: Day[] = [];
  const index = new Map<string, Day>();
  const today = new Date(nowMs);
  for (let i = days - 1; i >= 0; i--) {
    const d = new Date(today.getFullYear(), today.getMonth(), today.getDate() - i);
    const day: Day = { key: dayKey(d.getTime()), startMs: d.getTime(), costUsd: 0, tokens: 0, byFamily: {} };
    out.push(day);
    index.set(day.key, day);
  }
  for (const r of rows) {
    const day = index.get(dayKey(r.hourMs));
    if (!day) continue;
    day.costUsd += r.costUsd;
    day.tokens += r.tokens;
    const f = modelFamily(r.model);
    day.byFamily[f] = (day.byFamily[f] ?? 0) + r.costUsd;
  }
  return out;
}

export interface ProjectRow {
  project: string;
  tokens: number;
  costUsd: number;
  messages: number;
  lastMs: number;
}

/** Projects since `fromMs`, most expensive first. */
export function byProject(rows: HourRow[], fromMs: number): ProjectRow[] {
  const map = new Map<string, ProjectRow>();
  for (const r of rows) {
    if (r.hourMs < fromMs) continue;
    const p = map.get(r.project) ?? { project: r.project, tokens: 0, costUsd: 0, messages: 0, lastMs: 0 };
    p.tokens += r.tokens;
    p.costUsd += r.costUsd;
    p.messages += r.messages;
    p.lastMs = Math.max(p.lastMs, r.hourMs);
    map.set(r.project, p);
  }
  return [...map.values()].sort((a, b) => b.costUsd - a.costUsd || b.tokens - a.tokens);
}

export function sumCost(rows: HourRow[], fromMs: number): number {
  return rows.reduce((t, r) => (r.hourMs >= fromMs ? t + r.costUsd : t), 0);
}

/** "$0.42", "$12.30", "$1,204". */
export function formatUsd(v: number): string {
  if (v >= 1000) return `$${Math.round(v).toLocaleString("en-GB")}`;
  return `$${v.toFixed(2)}`;
}

/** "950", "12.3k", "4.1M", "250M", "1.2B". */
export function formatTokens(n: number): string {
  const units: [number, string][] = [
    [1e9, "B"],
    [1e6, "M"],
    [1e3, "k"],
  ];
  for (const [size, u] of units) {
    if (n >= size) {
      const v = n / size;
      return `${v >= 100 ? v.toFixed(0) : v.toFixed(1)}${u}`;
    }
  }
  return String(n);
}

/** Time left until `endMs`: "2 h 14 min", "9 min", "0 min". */
export function countdown(endMs: number, nowMs: number = Date.now()): string {
  const mins = Math.max(0, Math.ceil((endMs - nowMs) / 60000));
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  return h ? `${h} h ${m} min` : `${m} min`;
}

/** Share of a limit used, 0 to 1 (capped), or null with no limit set. */
export function share(used: number, limit: number | null): number | null {
  if (!limit || limit <= 0) return null;
  return Math.min(1, Math.max(0, used / limit));
}

/** Local time of day, "14:05". */
export function clockTime(ms: number): string {
  return new Date(ms).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit", hourCycle: "h23" });
}
