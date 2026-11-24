import type { Incident } from "../types";
import { duration, incidentStatusLabel, shortDateTime } from "../lib/format";
import { Chip, type Tone } from "./Chip";
import { Link } from "./Link";

interface Props {
  incidents: Incident[];
  empty: string;
  showUpdate?: boolean;
  kind?: "incident" | "maintenance";
}

/** Impact as a colour and a symbol; resolved incidents read as done. */
function chipFor(i: Incident): { tone: Tone; symbol: string } {
  if (i.resolvedAt) return { tone: "success", symbol: "✓" };
  switch (i.impact) {
    case "major":
    case "critical":
      return { tone: "danger", symbol: "!" };
    case "minor":
    case "maintenance":
      return { tone: "warning", symbol: "●" };
    default:
      return { tone: "plain", symbol: "●" };
  }
}

export function IncidentList({ incidents, empty, showUpdate = false, kind = "incident" }: Props) {
  if (incidents.length === 0) return <p className="meta">{empty}</p>;
  return (
    <ul className="rows incidents">
      {incidents.map((i) => {
        const when =
          kind === "maintenance" && i.scheduledFor
            ? `${shortDateTime(i.scheduledFor)}${i.scheduledUntil ? `, ${duration(i.scheduledFor, i.scheduledUntil)}` : ""}`
            : `${shortDateTime(i.createdAt)} · ${i.resolvedAt ? `lasted ${duration(i.createdAt, i.resolvedAt)}` : `${duration(i.createdAt, null)} so far`}`;
        const chip = chipFor(i);
        return (
          <li key={i.id} className="incident">
            <div className="incident-head">
              {i.url ? <Link href={i.url}>{i.name}</Link> : <span>{i.name}</span>}
              <Chip tone={chip.tone} symbol={chip.symbol}>
                {incidentStatusLabel(i.status)}
              </Chip>
            </div>
            <div className="meta">
              {when}
              {i.components.length > 0 && <> · {i.components.join(", ")}</>}
            </div>
            {showUpdate && i.latestUpdate && <p className="update">{i.latestUpdate}</p>}
          </li>
        );
      })}
    </ul>
  );
}
