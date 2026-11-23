import { useApp } from "../store";
import { useNow } from "../lib/useNow";
import { byModel, creditDays } from "../lib/credit";
import { formatUsd } from "../lib/usage";
import { relativeTime, shortDateTime } from "../lib/format";
import { CreditSummary } from "../components/CreditSummary";
import { DailyChart } from "../components/DailyChart";
import { Link } from "../components/Link";

export function ApiView({ onShowSettings }: { onShowSettings: () => void }) {
  const api = useApp((s) => s.api);
  const fetching = useApp((s) => s.apiFetching);
  const refresh = useApp((s) => s.refreshApi);
  const now = useNow();
  const s = api?.summary;

  return (
    <div className="stack">
      <section className="card">
        <div className="row">
          <h2 className="flush">API credit</h2>
          <span className="spacer" />
          {api?.configured && (
            <button type="button" className="ghost" disabled={fetching} onClick={() => void refresh()}>
              {fetching ? "Fetching…" : "Refresh"}
            </button>
          )}
        </div>
        <div className="gap-top">
          <CreditSummary api={api} now={now} onSetUp={onShowSettings} />
        </div>
        {api?.error && s && <p className="error small">{api.error} Showing the last figures.</p>}
        <p className="muted small note">
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
          <h2>Each day this period</h2>
          <DailyChart days={creditDays(s.rows, s.periodStartMs, s.periodEndMs, now)} label="API spend per day this credit period" />
        </section>
      )}

      {s && s.rows.length > 0 && (
        <section className="card">
          <h2>By model</h2>
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
                  <td>{m.model}</td>
                  <td className="num">{formatUsd(m.usd)}</td>
                  <td className="num muted">{s.spentUsd > 0 ? `${Math.round((m.usd / s.spentUsd) * 100)}%` : ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </div>
  );
}
