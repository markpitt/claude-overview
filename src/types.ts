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

// Claude Code usage. Keep in step with src-tauri/src/cc_service.rs and store.rs.

export interface HourRow {
  hourMs: number;
  model: string;
  project: string;
  tokens: number;
  output: number;
  costUsd: number;
  messages: number;
  sessions: number;
}

export interface Block {
  startMs: number;
  endMs: number;
  lastActivityMs: number;
  tokens: number;
  costUsd: number;
  messages: number;
}

export interface CcSettings {
  folderOverride: string | null;
  blockLimitUsd: number | null;
  weekLimitUsd: number | null;
  planPriceUsd: number | null;
}

export interface Totals {
  tokens: number;
  costUsd: number;
  messages: number;
}

export interface CcSummary {
  folder: string | null;
  folderExists: boolean;
  files: number;
  linesParsed: number;
  linesSkipped: number;
  lastScanMs: number;
  error: string | null;
  hourly: HourRow[];
  currentBlock: Block | null;
  lastBlock: Block | null;
  week: Totals;
  unpricedModels: string[];
  settings: CcSettings;
}
