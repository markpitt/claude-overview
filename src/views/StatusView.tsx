import { useApp } from "../store";
import { StatusHeader } from "../components/StatusHeader";
import { ComponentList } from "../components/ComponentList";
import { IncidentList } from "../components/IncidentList";
import { Link } from "../components/Link";

export function StatusView() {
  const status = useApp((s) => s.status);
  return (
    <div className="stack">
      <StatusHeader />
      <section className="card">
        <h2>Active incidents</h2>
        <IncidentList incidents={status?.activeIncidents ?? []} empty="No active incidents." showUpdate />
      </section>
      <section className="card">
        <h2>Services</h2>
        <ComponentList components={status?.components ?? []} />
      </section>
      <section className="card">
        <h2>Scheduled maintenance</h2>
        <IncidentList incidents={status?.maintenance ?? []} empty="Nothing scheduled." kind="maintenance" />
      </section>
      <section className="card">
        <h2>Recent incidents</h2>
        <IncidentList incidents={status?.recentIncidents ?? []} empty="No recent incidents." />
        <p className="small">
          Full history at <Link href="https://status.claude.com">status.claude.com</Link>
        </p>
      </section>
    </div>
  );
}
