import { useApp } from "../store";
import { StatusHeader } from "../components/StatusHeader";
import { IncidentList } from "../components/IncidentList";
import { Placeholder } from "./Placeholder";

export function OverviewView({ onShowStatus }: { onShowStatus: () => void }) {
  const status = useApp((s) => s.status);
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
        <Placeholder
          title="Max plan"
          milestone={2}
          detail="Claude Code tokens by day and project, and the current 5-hour block with its reset countdown."
        />
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
