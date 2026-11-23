import { useApp } from "../store";
import { StatusHeader } from "../components/StatusHeader";
import { IncidentList } from "../components/IncidentList";
import { PlanSummary } from "../components/PlanSummary";
import { useNow } from "../lib/useNow";
import { Placeholder } from "./Placeholder";

export function OverviewView({ onShowStatus, onShowCode }: { onShowStatus: () => void; onShowCode: () => void }) {
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
        <Placeholder
          title="API credit"
          milestone={3}
          detail="Spend against the $100 credit (resets on the 17th), burn rate and month-end forecast."
        />
        <Placeholder
          title="What's waiting"
          milestone={4}
          detail="Claude Code sessions waiting for you, batches in progress, and PRs to review."
        />
      </div>
    </div>
  );
}
