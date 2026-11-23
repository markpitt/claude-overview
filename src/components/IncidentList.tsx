import type { Incident } from "../types";
import { duration, incidentStatusLabel, shortDateTime } from "../lib/format";
import { Link } from "./Link";

interface Props {
  incidents: Incident[];
  empty: string;
  showUpdate?: boolean;
  kind?: "incident" | "maintenance";
}

export function IncidentList({ incidents, empty, showUpdate = false, kind = "incident" }: Props) {
  if (incidents.length === 0) return <p className="muted">{empty}</p>;
  return (
    <ul className="incidents">
      {incidents.map((i) => {
        const when =
          kind === "maintenance" && i.scheduledFor
            ? `${shortDateTime(i.scheduledFor)}${i.scheduledUntil ? `, ${duration(i.scheduledFor, i.scheduledUntil)}` : ""}`
            : `${shortDateTime(i.createdAt)} · ${i.resolvedAt ? `lasted ${duration(i.createdAt, i.resolvedAt)}` : `${duration(i.createdAt, null)} so far`}`;
        return (
          <li key={i.id} className={`incident impact-${i.impact}`}>
            <div className="incident-head">
              {i.url ? <Link href={i.url}>{i.name}</Link> : <span>{i.name}</span>}
              <span className="badge">{incidentStatusLabel(i.status)}</span>
            </div>
            <div className="muted small">
              {when}
              {i.components.length > 0 && <> · {i.components.join(", ")}</>}
            </div>
            {showUpdate && i.latestUpdate && <p className="update small">{i.latestUpdate}</p>}
          </li>
        );
      })}
    </ul>
  );
}
