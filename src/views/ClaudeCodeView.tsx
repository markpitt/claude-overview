import { useApp } from "../store";
import { useNow } from "../lib/useNow";
import { byLocalDay, byProject, DAY_MS, formatTokens, formatUsd } from "../lib/usage";
import { relativeTime } from "../lib/format";
import { PlanSummary } from "../components/PlanSummary";
import { DailyChart } from "../components/DailyChart";

export function ClaudeCodeView() {
  const cc = useApp((s) => s.cc);
  const scanning = useApp((s) => s.ccScanning);
  const rescan = useApp((s) => s.rescan);
  const now = useNow();

  if (!cc) {
    return <p className="muted">Reading Claude Code logs…</p>;
  }

  const days = byLocalDay(cc.hourly, 30, now);
  const projects = byProject(cc.hourly, now - 30 * DAY_MS);
  const noData = cc.hourly.length === 0;

  return (
    <div className="stack">
      <section className="card">
        <h2>Max plan</h2>
        <PlanSummary cc={cc} now={now} />
        <p className="muted small note">
          Costs are what this usage would cost at API prices. Anthropic does not publish Max plan limits, so the
          block window and allowances are estimates; set your own in Settings. claude.ai chats are not included.
        </p>
      </section>

      <section className="card">
        <h2>Last 30 days</h2>
        {noData ? <p className="muted">No Claude Code usage found yet.</p> : <DailyChart days={days} />}
      </section>

      {projects.length > 0 && (
        <section className="card">
          <h2>Projects, last 30 days</h2>
          <table className="table">
            <thead>
              <tr>
                <th>Project</th>
                <th className="num">Cost</th>
                <th className="num">Tokens</th>
                <th className="num">Responses</th>
                <th className="num">Last used</th>
              </tr>
            </thead>
            <tbody>
              {projects.map((p) => (
                <tr key={p.project}>
                  <td>{p.project}</td>
                  <td className="num">{formatUsd(p.costUsd)}</td>
                  <td className="num">{formatTokens(p.tokens)}</td>
                  <td className="num">{p.messages}</td>
                  <td className="num muted">{relativeTime(p.lastMs, now)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      <section className="card health">
        <div className="row">
          <h2>Log reader</h2>
          <span className="spacer" />
          <button type="button" className="ghost" disabled={scanning} onClick={() => void rescan()}>
            {scanning ? "Scanning…" : "Rescan"}
          </button>
        </div>
        <dl className="facts small">
          <dt>Folder</dt>
          <dd>
            <code>{cc.folder ?? "none"}</code>
            {!cc.folderExists && <span className="error"> (not found)</span>}
          </dd>
          <dt>Transcripts</dt>
          <dd>{cc.files} files</dd>
          <dt>Lines read</dt>
          <dd>
            {cc.linesParsed.toLocaleString()} since the app started
            {cc.linesSkipped > 0 && `, ${cc.linesSkipped} unreadable and skipped`}
          </dd>
          <dt>Last scan</dt>
          <dd>{relativeTime(cc.lastScanMs, now)}</dd>
          {cc.unpricedModels.length > 0 && (
            <>
              <dt>No price for</dt>
              <dd>{cc.unpricedModels.join(", ")} (counted in tokens, not cost)</dd>
            </>
          )}
        </dl>
        {cc.error && <p className="error small">{cc.error}</p>}
      </section>
    </div>
  );
}
