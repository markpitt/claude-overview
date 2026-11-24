import { useApp } from "../store";
import { StatusHeader } from "../components/StatusHeader";
import { ComponentList } from "../components/ComponentList";
import { IncidentList } from "../components/IncidentList";
import { CardHead } from "../components/CardHead";
import { Link } from "../components/Link";

export function StatusView() {
  const status = useApp((s) => s.status);
  return (
    <div className="stack">
      <StatusHeader />
      <section className="card">
        <CardHead title="Active incidents" />
        <IncidentList incidents={status?.activeIncidents ?? []} empty="No active incidents." showUpdate />
      </section>
      <section className="card">
        <CardHead title="Services" />
        <ComponentList components={status?.components ?? []} />
      </section>
      <section className="card">
        <CardHead title="Scheduled maintenance" />
        <IncidentList incidents={status?.maintenance ?? []} empty="Nothing scheduled." kind="maintenance" />
      </section>
      <section className="card">
        <CardHead title="Recent incidents">
          <Link href="https://status.claude.com">Full history</Link>
        </CardHead>
        <IncidentList incidents={status?.recentIncidents ?? []} empty="No recent incidents." />
      </section>
    </div>
  );
}
