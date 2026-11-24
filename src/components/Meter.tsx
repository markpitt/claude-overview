/** A thin bar showing how much of a limit is used, with the percentage beside it. Amber from 75%, red from 90%. */
export function Meter({ value, label }: { value: number; label: string }) {
  const pct = Math.round(value * 100);
  const tone = value >= 0.9 ? "danger" : value >= 0.75 ? "warning" : "success";
  return (
    <div className={`meter meter-${tone}`}>
      <div
        className="meter-track"
        role="meter"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={pct}
      >
        <div style={{ width: `${pct}%` }} />
      </div>
      <span className="meter-pct" aria-hidden="true">
        {pct}%
      </span>
    </div>
  );
}
