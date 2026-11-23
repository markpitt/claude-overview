import type { Level } from "../types";

/** "just now", "4 min ago", "3 h ago", "2 days ago". */
export function relativeTime(thenMs: number, nowMs: number = Date.now()): string {
  if (!thenMs) return "never";
  const s = Math.max(0, Math.round((nowMs - thenMs) / 1000));
  if (s < 45) return "just now";
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 36) return `${h} h ago`;
  const d = Math.round(h / 24);
  return `${d} day${d === 1 ? "" : "s"} ago`;
}

/** Plain words for a Statuspage component status. */
export function componentStatusLabel(status: string): string {
  switch (status) {
    case "operational":
      return "Operational";
    case "degraded_performance":
      return "Degraded";
    case "partial_outage":
      return "Partial outage";
    case "major_outage":
      return "Major outage";
    case "under_maintenance":
      return "Maintenance";
    default:
      return status.replace(/_/g, " ");
  }
}

/** Plain words for an incident's progress. */
export function incidentStatusLabel(status: string): string {
  const s = status.replace(/_/g, " ");
  return s.charAt(0).toUpperCase() + s.slice(1);
}

export function levelLabel(level: Level): string {
  switch (level) {
    case "ok":
      return "Operational";
    case "warn":
      return "Degraded";
    case "bad":
      return "Outage";
    default:
      return "Unknown";
  }
}

/** Duration between two ISO times, e.g. "1 h 18 min". */
export function duration(fromIso: string, toIso: string | null, nowMs: number = Date.now()): string {
  const from = Date.parse(fromIso);
  const to = toIso ? Date.parse(toIso) : nowMs;
  if (Number.isNaN(from) || Number.isNaN(to)) return "";
  const mins = Math.max(0, Math.round((to - from) / 60000));
  if (mins < 60) return `${mins} min`;
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  return m ? `${h} h ${m} min` : `${h} h`;
}

/** "Mon 23 Nov, 09:49" in the user's locale and time zone. */
export function shortDateTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    weekday: "short",
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}
