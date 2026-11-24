import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../store";
import { Lockup } from "../components/Brand";
import { Chip, LevelChip } from "../components/Chip";
import { Link } from "../components/Link";
import { ThemeSwitch } from "../components/ThemeSwitch";
import { OverviewView } from "./OverviewView";
import { StatusView } from "./StatusView";
import { SettingsView } from "./SettingsView";
import { WaitingView } from "./WaitingView";
import { waitingCounts } from "../lib/waiting";
import { displayShortcut, isMac } from "../lib/hotkey";
import { formatCount } from "../lib/format";
import type { HotkeyStatus, Level } from "../types";
import { ClaudeCodeView } from "./ClaudeCodeView";
import { ApiView } from "./ApiView";

type Tab = "overview" | "api" | "code" | "waiting" | "status" | "settings";

const TABS: { id: Tab; label: string }[] = [
  { id: "overview", label: "Overview" },
  { id: "api", label: "API" },
  { id: "code", label: "Claude Code" },
  { id: "waiting", label: "Waiting" },
  { id: "status", label: "Status" },
  { id: "settings", label: "Settings" },
];

const SHORT_LEVEL: Record<Level, string> = { ok: "OK", warn: "Degraded", bad: "Outage", unknown: "Unknown" };

/** The global shortcut as key caps; re-read when the tab changes, so a new one shows after Settings. */
function ShortcutKeys({ tab }: { tab: Tab }) {
  const [accel, setAccel] = useState<string | null>(null);
  useEffect(() => {
    invoke<HotkeyStatus>("get_hotkey")
      .then((s) => setAccel(s.accelerator))
      .catch(() => setAccel(null));
  }, [tab]);
  if (accel === null) return null;
  const mac = isMac();
  const shown = displayShortcut(accel, mac);
  if (!accel) return <p className="meta">Off</p>;
  const keys = mac ? [...shown] : shown.split("+");
  return (
    <p className="keys" aria-label={shown}>
      {keys.map((k, i) => (
        <code key={i}>{k}</code>
      ))}
    </p>
  );
}

export function MainWindow() {
  const [tab, setTab] = useState<Tab>("overview");
  const level = useApp((s) => s.status?.level ?? "unknown");
  const waitingTotal = useApp((s) => waitingCounts(s.waiting).total);
  const updateReady = useApp((s) => Boolean(s.update?.available));

  return (
    <div className="main-layout">
      <header className="app-bar">
        <Lockup />
        <ThemeSwitch />
      </header>
      <div className="main-body">
        <nav className="sidebar" aria-label="Sections">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              className={tab === t.id ? "tab active" : "tab"}
              aria-current={tab === t.id ? "page" : undefined}
              onClick={() => setTab(t.id)}
            >
              {t.label}
              {t.id === "status" && <LevelChip level={level}>{SHORT_LEVEL[level]}</LevelChip>}
              {t.id === "waiting" && waitingTotal > 0 && <Chip>{formatCount(waitingTotal)}</Chip>}
              {t.id === "settings" && updateReady && <Chip tone="success">Update</Chip>}
            </button>
          ))}
          <div className="sidebar-foot">
            <h2 className="section-label">Shortcut</h2>
            <ShortcutKeys tab={tab} />
            <h2 className="section-label">About</h2>
            <p className="meta">
              By Mark Pitt · <Link href="https://markpitt.dev">markpitt.dev</Link>
            </p>
          </div>
        </nav>
        <main className="content" key={tab}>
          <h1>{TABS.find((t) => t.id === tab)?.label}</h1>
          {tab === "overview" && (
            <OverviewView
              onShowStatus={() => setTab("status")}
              onShowCode={() => setTab("code")}
              onShowApi={() => setTab("api")}
              onShowSettings={() => setTab("settings")}
              onShowWaiting={() => setTab("waiting")}
            />
          )}
          {tab === "api" && <ApiView onShowSettings={() => setTab("settings")} />}
          {tab === "code" && <ClaudeCodeView />}
          {tab === "waiting" && <WaitingView onShowSettings={() => setTab("settings")} />}
          {tab === "status" && <StatusView />}
          {tab === "settings" && <SettingsView />}
        </main>
      </div>
    </div>
  );
}
