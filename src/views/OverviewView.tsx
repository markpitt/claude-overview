import { useApp } from "../store";
import { StatusHeader } from "../components/StatusHeader";
import { IncidentList } from "../components/IncidentList";
import { PlanSummary } from "../components/PlanSummary";
import { CreditSummary } from "../components/CreditSummary";
import { useNow } from "../lib/useNow";
import { WaitingList } from "../components/WaitingList";
import { anythingSetUp } from "../lib/waiting";

export function OverviewView({
  onShowStatus,
  onShowCode,
  onShowApi,
  onShowSettings,
  onShowWaiting,
}: {
  onShowStatus: () => void;
  onShowCode: () => void;
  onShowApi: () => void;
  onShowSettings: () => void;
  onShowWaiting: () => void;
}) {
  const waiting = useApp((s) => s.waiting);
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
      </div>
      <section className="card">
        <h2>What's waiting</h2>
        <WaitingList waiting={waiting} now={now} onSetUp={onShowSettings} />
        {waiting?.checked && anythingSetUp(waiting) && (
          <button type="button" className="ghost" onClick={onShowWaiting}>
            Everything waiting
          </button>
        )}
      </section>
    </div>
  );
}
