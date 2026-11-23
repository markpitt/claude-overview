import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../store";
import { StatusHeader } from "../components/StatusHeader";
import { IncidentList } from "../components/IncidentList";
import { LevelDot } from "../components/LevelDot";
import { componentStatusLabel } from "../lib/format";

/** The compact panel opened from the tray. */
export function Popover() {
  const status = useApp((s) => s.status);
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
      <section className="coming">
        <div>
          <span>Max plan</span>
          <span className="muted small">milestone 2</span>
        </div>
        <div>
          <span>API credit</span>
          <span className="muted small">milestone 3</span>
        </div>
        <div>
          <span>What's waiting</span>
          <span className="muted small">milestone 4</span>
        </div>
      </section>
    </div>
  );
}
