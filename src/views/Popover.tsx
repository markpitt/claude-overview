import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../store";
import { StatusHeader } from "../components/StatusHeader";
import { IncidentList } from "../components/IncidentList";
import { LevelDot } from "../components/LevelDot";
import { PlanSummary } from "../components/PlanSummary";
import { CreditSummary } from "../components/CreditSummary";
import { componentStatusLabel } from "../lib/format";
import { useNow } from "../lib/useNow";
import { WaitingList } from "../components/WaitingList";

/** The compact panel opened from the tray. */
export function Popover() {
  const status = useApp((s) => s.status);
  const cc = useApp((s) => s.cc);
  const api = useApp((s) => s.api);
  const waiting = useApp((s) => s.waiting);
  const now = useNow();
  const unhealthy = (status?.components ?? []).filter((c) => c.level !== "ok");

  return (
    <div className="popover">
      <header className="popover-top">
        <span className="brand">Claude Overview</span>
        <button type="button" className="ghost" onClick={() => void invoke("show_main")}>
          Open
        </button>
      </header>
      <StatusHeader />
      {(status?.activeIncidents.length ?? 0) > 0 && (
        <section>
          <h3>Active incidents</h3>
          <IncidentList incidents={status!.activeIncidents} empty="" />
        </section>
      )}
      {unhealthy.length > 0 && (
        <section>
          <h3>Affected services</h3>
          <ul className="components compact">
            {unhealthy.map((c) => (
              <li key={c.id}>
                <LevelDot level={c.level} />
                <span className="name">{c.name}</span>
                <span className={`state state-${c.level}`}>{componentStatusLabel(c.status)}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
      {cc && (
        <section>
          <h3>Max plan</h3>
          <PlanSummary cc={cc} now={now} compact />
        </section>
      )}
      {api?.configured && (
        <section>
          <h3>API credit</h3>
          <CreditSummary api={api} now={now} compact />
        </section>
      )}
      <section>
        <h3>What's waiting</h3>
        <WaitingList waiting={waiting} now={now} onSetUp={() => void invoke("show_main")} />
      </section>
    </div>
  );
}
