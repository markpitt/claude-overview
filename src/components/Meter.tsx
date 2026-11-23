/** A thin bar showing how much of a limit is used. Amber from 75%, red from 90%. */
export function Meter({ value, label }: { value: number; label: string }) {
  const pct = Math.round(value * 100);
  const tone = value >= 0.9 ? "bad" : value >= 0.75 ? "warn" : "ok";
  return (
    <div
      className={`meter meter-${tone}`}
      role="meter"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct}
    >
      <div style={{ width: `${pct}%` }} />
    </div>
  );
}
