// Shapes sent by the Rust core. Keep in step with src-tauri/src/status.rs.

export type Level = "ok" | "warn" | "bad" | "unknown";

export interface Component {
  id: string;
  name: string;
  status: string;
  level: Level;
  group: string | null;
}

export interface Incident {
  id: string;
  name: string;
  status: string;
  impact: string;
  createdAt: string;
  resolvedAt: string | null;
  scheduledFor: string | null;
  scheduledUntil: string | null;
  url: string | null;
  components: string[];
  latestUpdate: string | null;
}

export interface StatusSnapshot {
  level: Level;
  description: string;
  components: Component[];
  activeIncidents: Incident[];
  recentIncidents: Incident[];
  maintenance: Incident[];
  fetchedAtMs: number;
  error: string | null;
}
