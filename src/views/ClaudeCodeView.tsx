import { useApp } from "../store";
import { useNow } from "../lib/useNow";
import { byLocalDay, byProject, DAY_MS, formatTokens, formatUsd } from "../lib/usage";
import { formatCount, relativeTime } from "../lib/format";
import { PlanSummary } from "../components/PlanSummary";
import { DailyChart, peakUsd } from "../components/DailyChart";
import { CardHead, TabLink } from "../components/CardHead";

export function ClaudeCodeView() {
  const cc = useApp((s) => s.cc);
  const scanning = useApp((s) => s.ccScanning);
  const rescan = useApp((s) => s.rescan);
  const now = useNow();

  if (!cc) {
    return <p className="meta">Reading Claude Code logs…</p>;
  }

  const days = byLocalDay(cc.hourly, 30, now);
  const projects = byProject(cc.hourly, now - 30 * DAY_MS);
  const noData = cc.hourly.length === 0;
  const peak = peakUsd(days);

  return (
    <div className="stack">
      <p className="page-meta">
        From the transcripts in <code>{cc.folder ?? "none"}</code>
        {!cc.folderExists && <span className="error"> (not found)</span>} · {formatCount(cc.files)} files read ·{" "}
        {scanning ? <span>Scanning…</span> : <TabLink onClick={() => void rescan()}>Rescan</TabLink>}
      </p>

      <section className="card">
        <CardHead title="Max plan" />
        <PlanSummary cc={cc} now={now} />
        <p className="meta note">
          Costs are what this usage would cost at API prices. Anthropic does not publish Max plan limits, so the
          block window and allowances are estimates; set your own in Settings. claude.ai chats are not included.
        </p>
      </section>

      <section className="card">
        <CardHead title="Last 30 days">{peak > 0 && <span className="meta">peak {formatUsd(peak)} a day</span>}</CardHead>
        {noData ? <p className="meta">No Claude Code usage found yet.</p> : <DailyChart days={days} />}
      </section>

      {projects.length > 0 && (
        <section className="card">
          <CardHead title="Projects, last 30 days" />
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
                  <td className="mono">{p.project}</td>
                  <td className="num">{formatUsd(p.costUsd)}</td>
                  <td className="num">{formatTokens(p.tokens)}</td>
                  <td className="num">{formatCount(p.messages)}</td>
                  <td className="num meta">{relativeTime(p.lastMs, now)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      <section className="card health">
        <CardHead title="Log reader" />
        <dl className="facts">
          <dt>Folder</dt>
          <dd>
            <code>{cc.folder ?? "none"}</code>
            {!cc.folderExists && <span className="error"> (not found)</span>}
          </dd>
          <dt>Transcripts</dt>
          <dd>{formatCount(cc.files)} files</dd>
          <dt>Lines read</dt>
          <dd>
            {formatCount(cc.linesParsed)} since the app started
            {cc.linesSkipped > 0 && `, ${formatCount(cc.linesSkipped)} unreadable and skipped`}
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
        {cc.error && <p className="error">{cc.error}</p>}
      </section>
    </div>
  );
}
