import { useSyncExternalStore } from "react";
import { emit, listen } from "@tauri-apps/api/event";

export type ThemeChoice = "dark" | "light" | "system";

const KEY = "claude-overview.theme";
const EVENT = "theme-changed";
const query = () => window.matchMedia("(prefers-color-scheme: light)");

/** The saved choice; System when nothing is saved or storage is unavailable. */
export function savedTheme(): ThemeChoice {
  try {
    const v = localStorage.getItem(KEY);
    return v === "dark" || v === "light" ? v : "system";
  } catch {
    return "system";
  }
}

/** Puts dark or light on the root element. System follows the OS setting. */
export function applyTheme(choice: ThemeChoice): void {
  const resolved = choice === "system" ? (query().matches ? "light" : "dark") : choice;
  document.documentElement.dataset.theme = resolved;
}

let current: ThemeChoice = "system";
const subscribers = new Set<() => void>();

function set(choice: ThemeChoice) {
  current = choice;
  applyTheme(choice);
  subscribers.forEach((fn) => fn());
}

/**
 * Applies the saved theme and keeps it current: when the OS switches between
 * dark and light, and when the other window changes the choice.
 */
export function startTheme(): void {
  set(savedTheme());
  query().addEventListener("change", () => {
    if (current === "system") applyTheme("system");
  });
  // Both windows share storage; the event carries the change across at once.
  void listen<ThemeChoice>(EVENT, (e) => set(e.payload)).catch(() => {});
  window.addEventListener("focus", () => {
    const saved = savedTheme();
    if (saved !== current) set(saved);
  });
}

/** Saves the choice and tells every window. */
export function chooseTheme(choice: ThemeChoice): void {
  try {
    if (choice === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, choice);
  } catch {
    // Storage can be unavailable; the choice still applies until restart.
  }
  set(choice);
  void emit(EVENT, choice).catch(() => {});
}

function subscribe(fn: () => void): () => void {
  subscribers.add(fn);
  return () => {
    subscribers.delete(fn);
  };
}

export function useTheme(): ThemeChoice {
  return useSyncExternalStore(subscribe, () => current);
}
