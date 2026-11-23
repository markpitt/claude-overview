import { useEffect, useState } from "react";
import { useApp } from "../store";
import { relativeTime } from "../lib/format";
import { LevelDot } from "./LevelDot";

/** One-line headline: overall state, last check, refresh button. */
export function StatusHeader() {
  const status = useApp((s) => s.status);
  const refreshing = useApp((s) => s.refreshing);
  const refresh = useApp((s) => s.refresh);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(t);
  }, []);

  const level = status?.level ?? "unknown";
  return (
    <div className={`status-header level-${level}`}>
      <LevelDot level={level} size={12} />
      <div className="status-text">
        <strong>{status?.description ?? "Checking status…"}</strong>
        <span className="muted small">
          {status?.error
            ? status.fetchedAtMs
              ? `Couldn't refresh, showing data from ${relativeTime(status.fetchedAtMs, now)}`
              : `Couldn't reach status.claude.com: ${status.error}`
            : `Checked ${relativeTime(status?.fetchedAtMs ?? 0, now)}`}
        </span>
      </div>
      <button type="button" className="ghost" onClick={() => void refresh()} disabled={refreshing}>
        {refreshing ? "Refreshing…" : "Refresh"}
      </button>
    </div>
  );
}
