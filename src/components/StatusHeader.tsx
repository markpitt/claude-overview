import { useEffect, useState } from "react";
import { useApp } from "../store";
import { levelLabel, relativeTime } from "../lib/format";
import { LevelChip } from "./Chip";

/** The status banner: overall state, last check, refresh button. The pop-over puts the sentence in the chip. */
export function StatusHeader({ compact = false }: { compact?: boolean }) {
  const status = useApp((s) => s.status);
  const refreshing = useApp((s) => s.refreshing);
  const refresh = useApp((s) => s.refresh);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(t);
  }, []);

  const level = status?.level ?? "unknown";
  const sentence = status?.description ?? "Checking status…";
  const checked = status?.error
    ? status.fetchedAtMs
      ? `Couldn't refresh, showing data from ${relativeTime(status.fetchedAtMs, now)}`
      : `Couldn't reach status.claude.com: ${status.error}`
    : `Checked ${relativeTime(status?.fetchedAtMs ?? 0, now)}`;

  return (
    <div className={compact ? "status-banner compact" : "status-banner"}>
      {compact ? (
        <div className="status-text">
          <LevelChip level={level}>{sentence}</LevelChip>
          <span className="meta">{checked}</span>
        </div>
      ) : (
        <>
          <LevelChip level={level}>{levelLabel(level)}</LevelChip>
          <div className="status-text">
            <strong>{sentence}</strong>
            <span className="meta">{checked}</span>
          </div>
        </>
      )}
      <button type="button" onClick={() => void refresh()} disabled={refreshing}>
        {refreshing ? "Refreshing…" : "Refresh"}
      </button>
    </div>
  );
}
