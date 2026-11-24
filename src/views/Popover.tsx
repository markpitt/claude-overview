import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../store";
import { Lockup } from "../components/Brand";
import { StatusHeader } from "../components/StatusHeader";
import { IncidentList } from "../components/IncidentList";
import { ComponentList } from "../components/ComponentList";
import { PlanSummary } from "../components/PlanSummary";
import { CreditSummary } from "../components/CreditSummary";
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
        <Lockup size={20} />
        <button type="button" onClick={() => void invoke("show_main")}>
          Open
        </button>
      </header>
      <StatusHeader compact />
      {(status?.activeIncidents.length ?? 0) > 0 && (
        <section>
          <h2 className="section-label">Active incidents</h2>
          <IncidentList incidents={status!.activeIncidents} empty="" />
        </section>
      )}
      {unhealthy.length > 0 && (
        <section>
          <h2 className="section-label">Affected services</h2>
          <ComponentList components={unhealthy} compact />
        </section>
      )}
      {cc && (
        <section>
          <h2 className="section-label">Max plan</h2>
          <PlanSummary cc={cc} now={now} compact />
        </section>
      )}
      {api?.configured && (
        <section>
          <h2 className="section-label">API credit</h2>
          <CreditSummary api={api} now={now} compact />
        </section>
      )}
      <section>
        <h2 className="section-label">What's waiting</h2>
        <WaitingList waiting={waiting} now={now} onSetUp={() => void invoke("show_main")} />
      </section>
    </div>
  );
}
