import { useId } from "react";
import type { Day } from "../lib/usage";
import { formatUsd } from "../lib/usage";
import { dayMonth, shortDate } from "../lib/format";

const W = 600;
const H = 104;
const PAD_BOTTOM = 2;
/** Bottom to top: a tonal ramp from ink down, never a hue per series. */
const ORDER = ["opus", "sonnet", "fable", "haiku", "mythos", "other"] as const;
const HATCHED = new Set(["haiku", "mythos"]);

/** The highest day in the chart, for the card heading. */
export function peakUsd(days: Day[]): number {
  return Math.max(0, ...days.map((d) => d.costUsd));
}

/** Stacked greyscale bars of dollars per day, one tone per model family, with the legend in words. */
export function DailyChart({ days, label = "API-price cost per day for the last 30 days" }: { days: Day[]; label?: string }) {
  const uid = useId().replace(/:/g, "");
  const max = Math.max(peakUsd(days), 0.01);
  const slot = W / days.length;
  const bar = Math.max(2, slot * 0.7);
  const used = ORDER.filter((f) => days.some((d) => (d.byFamily[f] ?? 0) > 0));
  const fill = (f: string) => (HATCHED.has(f) ? `url(#${uid}-${f})` : undefined);
  const summary = `${label}. Peak ${formatUsd(peakUsd(days))} a day.${used.length ? ` Families: ${used.join(", ")}.` : ""}`;

  return (
    <figure className="chart">
      <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={summary}>
        <defs>
          {used
            .filter((f) => HATCHED.has(f))
            .map((f) => (
              <pattern
                key={f}
                id={`${uid}-${f}`}
                patternUnits="userSpaceOnUse"
                width="4"
                height="4"
                patternTransform={`rotate(${f === "haiku" ? 45 : -45})`}
              >
                <rect width="4" height="4" className="hatch-bg" />
                <line x1="0" y1="0" x2="0" y2="4" className={`hatch-line hatch-${f}`} />
              </pattern>
            ))}
        </defs>
        <line x1={0} x2={W} y1={H - PAD_BOTTOM + 0.5} y2={H - PAD_BOTTOM + 0.5} className="axis" />
        {days.map((d, i) => {
          let y = H - PAD_BOTTOM;
          const x = i * slot + (slot - bar) / 2;
          return (
            <g key={d.key}>
              <title>
                {shortDate(d.startMs)}: {formatUsd(d.costUsd)}
              </title>
              {used.map((f) => {
                const v = d.byFamily[f] ?? 0;
                if (v <= 0) return null;
                const h = (v / max) * (H - PAD_BOTTOM - 6);
                // A 1px gap between stacked segments.
                const drawn = Math.max(0.5, h - 1);
                y -= h;
                return <rect key={f} x={x} y={y + (h - drawn)} width={bar} height={drawn} className={`fam fam-${f}`} fill={fill(f)} />;
              })}
</g>
          );
        })}
      </svg>
      {/* Ticks in HTML so they stay 11px whatever the chart's width. */}
      <div className="ticks" aria-hidden="true">
        {days.map((d, i) =>
          i === days.length - 1 || (i % 7 === 0 && days.length - 1 - i >= 3) ? (
            <span key={d.key} style={{ left: `${((i + 0.5) / days.length) * 100}%` }} className={i === 0 ? "first" : i === days.length - 1 ? "last" : undefined}>
              {dayMonth(d.startMs)}
            </span>
          ) : null,
        )}
      </div>
      <figcaption className="legend">
        {used.map((f) => (
          <span key={f}>
            <i className={`swatch fam-${f}`} aria-hidden="true" />
            {f}
          </span>
        ))}
      </figcaption>
    </figure>
  );
}
