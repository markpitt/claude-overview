import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CcSettings, CcSummary, StatusSnapshot } from "./types";

interface AppStore {
  status: StatusSnapshot | null;
  refreshing: boolean;
  refresh: () => Promise<void>;
  cc: CcSummary | null;
  ccScanning: boolean;
  rescan: () => Promise<void>;
  saveCcSettings: (s: CcSettings) => Promise<void>;
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
  rescan: async () => {
    set({ ccScanning: true });
    await invoke("refresh_claude_code");
  },
  saveCcSettings: async (settings) => {
    set({ ccScanning: true });
    try {
      await invoke("save_cc_settings", { settings });
    } catch (e) {
      set({ ccScanning: false });
      throw e;
    }
  },
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
}
