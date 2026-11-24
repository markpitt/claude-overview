import { useApp } from "../store";
import { useNow } from "../lib/useNow";
import { byModel, creditDays } from "../lib/credit";
import { formatUsd } from "../lib/usage";
import { relativeTime, shortDateTime } from "../lib/format";
import { CreditSummary } from "../components/CreditSummary";
import { DailyChart, peakUsd } from "../components/DailyChart";
import { CardHead } from "../components/CardHead";
import { Link } from "../components/Link";

export function ApiView({ onShowSettings }: { onShowSettings: () => void }) {
  const api = useApp((s) => s.api);
  const fetching = useApp((s) => s.apiFetching);
  const refresh = useApp((s) => s.refreshApi);
  const now = useNow();
  const s = api?.summary;
  const days = s ? creditDays(s.rows, s.periodStartMs, s.periodEndMs, now) : [];
  const peak = peakUsd(days);

  return (
    <div className="stack">
      <section className="card">
        <CardHead title="API credit">
          {api?.configured && (
            <button type="button" disabled={fetching} onClick={() => void refresh()}>
              {fetching ? "Fetching…" : "Refresh"}
            </button>
          )}
        </CardHead>
        <CreditSummary api={api} now={now} onSetUp={onShowSettings} />
        {api?.error && s && <p className="error">{api.error} Showing the last figures.</p>}
        <p className="meta note">
          Spend at your organisation's prices before credits, from the Usage and Cost Analytics API.{" "}
          {s?.refreshedMs
            ? `Anthropic's data runs to ${shortDateTime(new Date(s.refreshedMs).toISOString())}; it lags 4 to 24 hours.`
            : "Anthropic's data lags 4 to 24 hours."}{" "}
          {api && api.fetchedAtMs > 0 && `Checked ${relativeTime(api.fetchedAtMs, now)}.`} Check the{" "}
          <Link href="https://platform.claude.com">Claude Console</Link> for the exact balance.
        </p>
      </section>

      {s && (
        <section className="card">
          <CardHead title="Each day this period">
            {peak > 0 && <span className="meta">peak {formatUsd(peak)} a day</span>}
          </CardHead>
          <DailyChart days={days} label="API spend per day this credit period" />
        </section>
      )}

      {s && s.rows.length > 0 && (
        <section className="card">
          <CardHead title="By model" />
          <table className="table">
            <thead>
              <tr>
                <th>Model</th>
                <th className="num">Spend</th>
                <th className="num">Share</th>
              </tr>
            </thead>
            <tbody>
              {byModel(s.rows).map((m) => (
                <tr key={m.model}>
                  <td className="mono">{m.model}</td>
                  <td className="num">{formatUsd(m.usd)}</td>
                  <td className="num meta">{s.spentUsd > 0 ? `${Math.round((m.usd / s.spentUsd) * 100)}%` : ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </div>
  );
}
