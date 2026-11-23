import { useState } from "react";
import { useApp } from "../store";
import { LevelDot } from "../components/LevelDot";
import { OverviewView } from "./OverviewView";
import { StatusView } from "./StatusView";
import { SettingsView } from "./SettingsView";
import { WaitingView } from "./WaitingView";
import { waitingCounts } from "../lib/waiting";
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

export function MainWindow() {
  const [tab, setTab] = useState<Tab>("overview");
  const level = useApp((s) => s.status?.level ?? "unknown");
  const waitingTotal = useApp((s) => waitingCounts(s.waiting).total);

  return (
    <div className="main-layout">
      <nav className="sidebar" aria-label="Sections">
        <div className="brand">Claude Overview</div>
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            className={tab === t.id ? "tab active" : "tab"}
            aria-current={tab === t.id ? "page" : undefined}
            onClick={() => setTab(t.id)}
          >
            {t.label}
            {t.id === "status" && <LevelDot level={level} size={8} />}
            {t.id === "waiting" && waitingTotal > 0 && <span className="count">{waitingTotal}</span>}
          </button>
        ))}
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
        {tab === "api" && (
          <ApiView onShowSettings={() => setTab("settings")} />
        )}
        {tab === "code" && <ClaudeCodeView />}
        {tab === "waiting" && (
          <WaitingView onShowSettings={() => setTab("settings")} />
        )}
        {tab === "status" && <StatusView />}
        {tab === "settings" && <SettingsView />}
      </main>
    </div>
  );
}
