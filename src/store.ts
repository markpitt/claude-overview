import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { StatusSnapshot } from "./types";

interface AppStore {
  status: StatusSnapshot | null;
  refreshing: boolean;
  refresh: () => Promise<void>;
}

export const useApp = create<AppStore>((set) => ({
  status: null,
  refreshing: false,
  refresh: async () => {
    set({ refreshing: true });
    await invoke("refresh_status");
  },
}));

let started = false;

/** Loads the current snapshot and subscribes to pushes from Rust. */
export async function startStore(): Promise<void> {
  if (started) return;
  started = true;
  await listen<StatusSnapshot>("status-updated", (e) => {
    useApp.setState({ status: e.payload, refreshing: false });
  });
  const current = await invoke<StatusSnapshot>("get_status");
  // A push may have landed first; keep the newer one.
  const existing = useApp.getState().status;
  if (!existing || existing.fetchedAtMs <= current.fetchedAtMs) {
    useApp.setState({ status: current });
  }
}
