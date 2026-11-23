import { useApp } from "../store";
import { StatusHeader } from "../components/StatusHeader";
import { IncidentList } from "../components/IncidentList";
import { PlanSummary } from "../components/PlanSummary";
import { CreditSummary } from "../components/CreditSummary";
import { useNow } from "../lib/useNow";
import { Placeholder } from "./Placeholder";

export function OverviewView({
  onShowStatus,
  onShowCode,
  onShowApi,
  onShowSettings,
}: {
  onShowStatus: () => void;
  onShowCode: () => void;
  onShowApi: () => void;
  onShowSettings: () => void;
}) {
  const api = useApp((s) => s.api);
  const status = useApp((s) => s.status);
  const cc = useApp((s) => s.cc);
  const now = useNow();
  const active = status?.activeIncidents ?? [];
  return (
    <div className="stack">
      <StatusHeader />
      {active.length > 0 && (
        <section className="card">
          <h2>Active incidents</h2>
          <IncidentList incidents={active} empty="" showUpdate />
          <button type="button" className="ghost" onClick={onShowStatus}>
            All services
          </button>
        </section>
      )}
      <div className="grid">
        {cc ? (
          <section className="card">
            <h2>Max plan</h2>
            <PlanSummary cc={cc} now={now} compact />
            <button type="button" className="ghost" onClick={onShowCode}>
              Claude Code usage
            </button>
          </section>
        ) : (
          <section className="card">
            <h2>Max plan</h2>
            <p className="muted">Reading Claude Code logs…</p>
          </section>
        )}
        <section className="card">
          <h2>API credit</h2>
          <CreditSummary api={api} now={now} compact onSetUp={onShowSettings} />
          {api?.summary && (
            <button type="button" className="ghost" onClick={onShowApi}>
              API spend
            </button>
          )}
        </section>
        <Placeholder
          title="What's waiting"
          milestone={4}
          detail="Claude Code sessions waiting for you, batches in progress, and PRs to review."
        />
      </div>
    </div>
  );
}
