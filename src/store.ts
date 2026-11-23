import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ApiSettings, ApiSnapshot, CcSettings, CcSummary, StatusSnapshot, UpdateSnapshot, WaitingSnapshot } from "./types";

interface AppStore {
  status: StatusSnapshot | null;
  refreshing: boolean;
  refresh: () => Promise<void>;
  cc: CcSummary | null;
  ccScanning: boolean;
  rescan: () => Promise<void>;
  saveCcSettings: (s: CcSettings) => Promise<void>;
  api: ApiSnapshot | null;
  apiFetching: boolean;
  refreshApi: () => Promise<void>;
  saveApiSettings: (s: ApiSettings) => Promise<void>;
  setAdminKey: (key: string | null) => Promise<void>;
  waiting: WaitingSnapshot | null;
  waitingBusy: boolean;
  refreshWaiting: () => Promise<void>;
  setWaitingKey: (kind: "api" | "github", value: string | null) => Promise<void>;
  setHooks: (on: boolean) => Promise<void>;
  update: UpdateSnapshot | null;
}

/** Marks a request busy, and clears the flag if it fails (a push clears it on success). */
async function busy(set: (s: Partial<AppStore>) => void, flag: "ccScanning" | "apiFetching" | "waitingBusy", run: () => Promise<unknown>) {
  set({ [flag]: true });
  try {
    await run();
  } catch (e) {
    set({ [flag]: false });
    throw e;
  }
}

export const useApp = create<AppStore>((set) => ({
  status: null,
  refreshing: false,
  refresh: async () => {
    set({ refreshing: true });
    await invoke("refresh_status");
  },
  cc: null,
  ccScanning: false,
  rescan: () => busy(set, "ccScanning", () => invoke("refresh_claude_code")),
  saveCcSettings: (settings) => busy(set, "ccScanning", () => invoke("save_cc_settings", { settings })),
  api: null,
  apiFetching: false,
  refreshApi: () => busy(set, "apiFetching", () => invoke("refresh_api_credit")),
  saveApiSettings: (settings) => busy(set, "apiFetching", () => invoke("save_api_settings", { settings })),
  setAdminKey: (key) => busy(set, "apiFetching", () => invoke("set_admin_key", { key })),
  waiting: null,
  waitingBusy: false,
  refreshWaiting: () => busy(set, "waitingBusy", () => invoke("refresh_waiting")),
  setWaitingKey: (kind, value) => busy(set, "waitingBusy", () => invoke("set_waiting_key", { kind, value })),
  setHooks: (on) => busy(set, "waitingBusy", () => invoke("set_hooks", { on })),
  update: null,
}));

let started = false;

/** Loads the current snapshots and subscribes to pushes from Rust. */
export async function startStore(): Promise<void> {
  if (started) return;
  started = true;
  await listen<StatusSnapshot>("status-updated", (e) => {
    useApp.setState({ status: e.payload, refreshing: false });
  });
  await listen<CcSummary>("claude-code-updated", (e) => {
    useApp.setState({ cc: e.payload, ccScanning: false });
  });
  await listen<ApiSnapshot>("api-credit-updated", (e) => {
    useApp.setState({ api: e.payload, apiFetching: false });
  });
  await listen<WaitingSnapshot>("waiting-updated", (e) => {
    useApp.setState({ waiting: e.payload, waitingBusy: false });
  });
  await listen<UpdateSnapshot>("update-updated", (e) => {
    useApp.setState({ update: e.payload });
  });
  const current = await invoke<StatusSnapshot>("get_status");
  // A push may have landed first; keep the newer one.
  const existing = useApp.getState().status;
  if (!existing || existing.fetchedAtMs <= current.fetchedAtMs) {
    useApp.setState({ status: current });
  }
  const cc = await invoke<CcSummary | null>("get_claude_code").catch(() => null);
  const ccExisting = useApp.getState().cc;
  if (cc && (!ccExisting || ccExisting.lastScanMs <= cc.lastScanMs)) {
    useApp.setState({ cc });
  }
  const api = await invoke<ApiSnapshot>("get_api_credit").catch(() => null);
  if (api && !useApp.getState().api) {
    useApp.setState({ api });
  }
  const waiting = await invoke<WaitingSnapshot>("get_waiting").catch(() => null);
  if (waiting && !useApp.getState().waiting) {
    useApp.setState({ waiting });
  }
  const update = await invoke<UpdateSnapshot>("get_update").catch(() => null);
  if (update && !useApp.getState().update) {
    useApp.setState({ update });
  }
}
