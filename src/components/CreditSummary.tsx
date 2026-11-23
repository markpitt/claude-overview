import type { ApiSnapshot } from "../types";
import { daysUntil, utcDate } from "../lib/credit";
import { formatUsd, share } from "../lib/usage";
import { Meter } from "./Meter";

/** Spend against the monthly credit, with the forecast. Renders a prompt when no key is set. */
export function CreditSummary({
  api,
  now,
  compact = false,
  onSetUp,
}: {
  api: ApiSnapshot | null;
  now: number;
  compact?: boolean;
  onSetUp?: () => void;
}) {
  if (!api || !api.checked) return <p className="muted small">Checking…</p>;
  if (!api.configured) {
    return (
      <div className="small">
        <p className="muted">Add an Admin API key to see spend against your credit.</p>
        {onSetUp && (
          <button type="button" className="ghost" onClick={onSetUp}>
            Add key
          </button>
        )}
      </div>
    );
  }
  const s = api.summary;
  if (!s) {
    return api.error ? <p className="error small">{api.error}</p> : <p className="muted small">Fetching…</p>;
  }
  const used = share(s.spentUsd, s.allowanceUsd) ?? 0;
  const left = s.allowanceUsd - s.spentUsd;
  const reset = daysUntil(s.periodEndMs, now);

  return (
    <div className={compact ? "plan compact" : "plan"}>
      <div className="plan-item">
        <div className="plan-head">
          <span>This month</span>
          <span className="muted small">
            resets in {reset} day{reset === 1 ? "" : "s"}
          </span>
        </div>
        <div className="plan-figure">
          {formatUsd(s.spentUsd)}
          <span className="muted"> of {formatUsd(s.allowanceUsd)}</span>
        </div>
        <Meter value={used} label="Share of the API credit used" />
        <div className="muted small plan-detail">
          {left >= 0 ? `${formatUsd(left)} left` : `${formatUsd(-left)} over`}
        </div>
      </div>
      {!compact && (
        <>
          <div className="plan-item">
            <div className="plan-head">
              <span>Burn rate</span>
              <span className="muted small">this period</span>
            </div>
            <div className="plan-figure">
              {formatUsd(s.burnPerDayUsd)}
              <span className="muted small"> a day</span>
            </div>
          </div>
          <div className="plan-item">
            <div className="plan-head">
              <span>Forecast</span>
              <span className="muted small">at {utcDate(s.periodEndMs)}</span>
            </div>
            <div className={s.forecastUsd > s.allowanceUsd ? "plan-figure state-bad" : "plan-figure"}>
              {formatUsd(s.forecastUsd)}
            </div>
            <div className="muted small plan-detail">
              {s.runsOutMs ? `Runs out around ${utcDate(s.runsOutMs)}` : "Within the credit at this rate"}
            </div>
          </div>
        </>
      )}
      {compact && s.runsOutMs && <div className="small state-bad">Runs out around {utcDate(s.runsOutMs)}</div>}
    </div>
  );
}
