import { useApp } from "../store";
import { StatusHeader } from "../components/StatusHeader";
import { IncidentList } from "../components/IncidentList";
import { PlanSummary } from "../components/PlanSummary";
import { CreditSummary } from "../components/CreditSummary";
import { useNow } from "../lib/useNow";
import { WaitingList } from "../components/WaitingList";
import { anythingSetUp } from "../lib/waiting";
import { CardHead, TabLink } from "../components/CardHead";

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
          <CardHead title="Active incidents">
            <TabLink onClick={onShowStatus}>All services</TabLink>
          </CardHead>
          <IncidentList incidents={active} empty="" showUpdate />
        </section>
      )}
      <div className="grid">
        <section className="card">
          <CardHead title="Max plan">{cc && <TabLink onClick={onShowCode}>Claude Code usage</TabLink>}</CardHead>
          {cc ? <PlanSummary cc={cc} now={now} compact /> : <p className="meta">Reading Claude Code logs…</p>}
        </section>
        <section className="card">
          <CardHead title="API credit">{api?.summary && <TabLink onClick={onShowApi}>API spend</TabLink>}</CardHead>
          <CreditSummary api={api} now={now} compact forecast onSetUp={onShowSettings} />
        </section>
      </div>
      <section className="card">
        <CardHead title="What's waiting">
          {waiting?.checked && anythingSetUp(waiting) && <TabLink onClick={onShowWaiting}>Everything waiting</TabLink>}
        </CardHead>
        <WaitingList waiting={waiting} now={now} onSetUp={onShowSettings} />
      </section>
    </div>
  );
}
