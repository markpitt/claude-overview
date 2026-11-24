import type { CcSummary } from "../types";
import { formatCount } from "../lib/format";
import { clockTime, countdown, DAY_MS, formatTokens, formatUsd, share, sumCost } from "../lib/usage";
import { Meter } from "./Meter";

/** The current 5-hour block, the last 7 days and the last 30 days, against your own limits. */
export function PlanSummary({ cc, now, compact = false }: { cc: CcSummary; now: number; compact?: boolean }) {
  const { settings } = cc;
  const block = cc.currentBlock;
  const blockShare = block ? share(block.costUsd, settings.blockLimitUsd) : null;
  const weekShare = share(cc.week.costUsd, settings.weekLimitUsd);
  const month = sumCost(cc.hourly, now - 30 * DAY_MS);

  return (
    <div className={compact ? "items compact" : "items columns"}>
      <div className="item">
        <div className="item-head">
          <span>5-hour block</span>
          {block ? (
            <span className="meta" title={`Resets at ${clockTime(block.endMs)}`}>
              resets in {countdown(block.endMs, now)}
            </span>
          ) : (
            <span className="meta">no block running</span>
          )}
        </div>
        {block ? (
          <>
            <div className="figure">
              {formatUsd(block.costUsd)}
              {settings.blockLimitUsd ? <span className="of"> of {formatUsd(settings.blockLimitUsd)}</span> : null}
            </div>
            {blockShare !== null && <Meter value={blockShare} label="Share of the 5-hour block used" />}
            {!compact && (
              <div className="meta detail">
                {formatTokens(block.tokens)} tokens · {formatCount(block.messages)} responses
              </div>
            )}
          </>
        ) : (
          !compact &&
          cc.lastBlock && (
            <p className="meta detail">
              Last block ended {clockTime(cc.lastBlock.endMs)}: {formatUsd(cc.lastBlock.costUsd)}.
            </p>
          )
        )}
      </div>

      <div className="item">
        <div className="item-head">
          <span>Last 7 days</span>
          {!compact && <span className="meta">{formatCount(cc.week.messages)} responses</span>}
        </div>
        <div className="figure">
          {formatUsd(cc.week.costUsd)}
          {settings.weekLimitUsd ? <span className="of"> of {formatUsd(settings.weekLimitUsd)}</span> : null}
        </div>
        {weekShare !== null && <Meter value={weekShare} label="Share of the weekly allowance used" />}
      </div>

      {!compact && (
        <div className="item">
          <div className="item-head">
            <span>Last 30 days</span>
            <span className="meta">at API prices</span>
          </div>
          <div className="figure">{formatUsd(month)}</div>
          {settings.planPriceUsd ? (
            <div className="meta detail">
              {(month / settings.planPriceUsd).toFixed(1)}× the {formatUsd(settings.planPriceUsd)} a month you pay
            </div>
          ) : null}
        </div>
      )}
    </div>
  );
}
