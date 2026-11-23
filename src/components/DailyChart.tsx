import type { Day } from "../lib/usage";
import { FAMILIES, formatUsd } from "../lib/usage";

const W = 600;
const H = 160;
const PAD_BOTTOM = 18;

/** Stacked bars of API-price dollars per local day, coloured by model family. */
export function DailyChart({ days }: { days: Day[] }) {
  const max = Math.max(...days.map((d) => d.costUsd), 0.01);
  const slot = W / days.length;
  const bar = Math.max(2, slot * 0.7);
  const used = FAMILIES.filter((f) => days.some((d) => (d.byFamily[f] ?? 0) > 0));

  return (
    <figure className="chart">
      <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label="API-price cost per day for the last 30 days">
        <line x1={0} x2={W} y1={H - PAD_BOTTOM} y2={H - PAD_BOTTOM} className="axis" />
        {days.map((d, i) => {
          let y = H - PAD_BOTTOM;
          const x = i * slot + (slot - bar) / 2;
          const date = new Date(d.startMs);
          return (
            <g key={d.key}>
              <title>
                {date.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" })}:{" "}
                {formatUsd(d.costUsd)}
              </title>
              {used.map((f) => {
                const v = d.byFamily[f] ?? 0;
                if (v <= 0) return null;
                const h = (v / max) * (H - PAD_BOTTOM - 6);
                y -= h;
                return <rect key={f} x={x} y={y} width={bar} height={h} className={`fam fam-${f}`} />;
              })}
              {(i === days.length - 1 || (i % 7 === 0 && days.length - 1 - i >= 3)) && (
                <text x={i * slot + slot / 2} y={H - 4} textAnchor="middle" className="tick">
                  {date.getDate()}
                </text>
              )}
            </g>
          );
        })}
      </svg>
      <figcaption className="legend">
        {used.map((f) => (
          <span key={f}>
            <i className={`swatch fam-${f}`} />
            {f}
          </span>
        ))}
        {max > 0.01 && <span className="muted">peak {formatUsd(max)} a day</span>}
      </figcaption>
    </figure>
  );
}
