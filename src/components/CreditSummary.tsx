import type { ApiSnapshot } from "../types";
import { daysUntil, utcDate } from "../lib/credit";
import { clockTime, formatUsd, share } from "../lib/usage";
import { Chip } from "./Chip";
import { Meter } from "./Meter";

/**
 * Spend against the monthly credit, with the forecast. Renders a prompt when no key is set.
 * `compact` stacks the items for a card or the pop-over; `forecast` adds the forecast item there.
 */
export function CreditSummary({
  api,
  now,
  compact = false,
  forecast = false,
  onSetUp,
}: {
  api: ApiSnapshot | null;
  now: number;
  compact?: boolean;
  forecast?: boolean;
  onSetUp?: () => void;
}) {
  if (!api || !api.checked) return <p className="meta">Checking…</p>;
  if (!api.configured) {
    return (
      <div className="prompt">
        <p className="meta">Add an Admin API key to see spend against your credit.</p>
        {onSetUp && (
          <button type="button" onClick={onSetUp}>
            Add key
          </button>
        )}
      </div>
    );
  }
  const s = api.summary;
  if (!s) {
    return api.error ? <p className="error">{api.error}</p> : <p className="meta">Fetching…</p>;
  }
  const used = share(s.spentUsd, s.allowanceUsd) ?? 0;
  const left = s.allowanceUsd - s.spentUsd;
  const reset = daysUntil(s.periodEndMs, now);
  const leftText = left >= 0 ? `${formatUsd(left)} left` : `${formatUsd(-left)} over`;
  const runsOut = s.runsOutMs ? (
    <Chip tone="danger" symbol="!">
      Runs out around {utcDate(s.runsOutMs)}
    </Chip>
  ) : null;
  const showForecast = !compact || forecast;

  return (
    <div className={compact ? "items compact" : "items columns"}>
      <div className="item">
        <div className="item-head">
          <span>This month</span>
          <span className="meta">
            resets in {reset} day{reset === 1 ? "" : "s"}
          </span>
        </div>
        <div className="figure">
          {formatUsd(s.spentUsd)}
          <span className="of"> of {formatUsd(s.allowanceUsd)}</span>
        </div>
        <Meter value={used} label="Share of the API credit used" />
        <div className="meta detail">
          {leftText}
          {compact && s.refreshedMs ? ` · data refreshed ${clockTime(s.refreshedMs)}` : ""}
        </div>
        {compact && !forecast && runsOut && <div className="detail">{runsOut}</div>}
      </div>
      {!compact && (
        <div className="item">
          <div className="item-head">
            <span>Burn rate</span>
            <span className="meta">this period</span>
          </div>
          <div className="figure">
            {formatUsd(s.burnPerDayUsd)}
            <span className="of"> a day</span>
          </div>
        </div>
      )}
      {showForecast && (
        <div className="item">
          <div className="item-head">
            <span>Forecast</span>
            <span className="meta">at {utcDate(s.periodEndMs)}</span>
          </div>
          <div className="figure">{formatUsd(s.forecastUsd)}</div>
          <div className="detail">{runsOut ?? <span className="meta">Within the credit at this rate</span>}</div>
        </div>
      )}
    </div>
  );
}
