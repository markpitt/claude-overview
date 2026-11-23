import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../store";
import { useNow } from "../lib/useNow";
import { relativeTime } from "../lib/format";
import { Link } from "./Link";

const RELEASES = "https://github.com/markpitt/claude-overview/releases";

function megabytes(n: number): string {
  return (n / 1_048_576).toFixed(1);
}

/** The installed version, the daily check and installing a new release. */
export function UpdatesPanel() {
  const u = useApp((s) => s.update);
  const now = useNow(30_000);
  const [error, setError] = useState<string | null>(null);
  if (!u) return <p className="muted small">Loading…</p>;

  async function install() {
    setError(null);
    try {
      await invoke("install_update");
    } catch (e) {
      setError(String(e));
    }
  }

  const installing = u.progress !== null;
  // Before the first release there is nothing to fetch; that isn't a fault.
  const noRelease = !error && Boolean(u.error?.startsWith("No published release"));
  const [done, total] = u.progress ?? [0, null];

  return (
    <div className="form">
      {u.available ? (
        <div className="update-ready">
          <p className="flush">
            <strong>Version {u.available} is available.</strong> You have {u.current}.
          </p>
          {u.notes && <p className="small muted clip-lines">{u.notes}</p>}
          <div className="row">
            <button type="button" disabled={installing} onClick={() => void install()}>
              {installing
                ? total
                  ? `Downloading… ${megabytes(done)} of ${megabytes(total)} MB`
                  : `Downloading… ${megabytes(done)} MB`
                : "Install and restart"}
            </button>
            <Link href={`${RELEASES}/tag/v${u.available}`}>What's new</Link>
          </div>
        </div>
      ) : (
        <p className="flush">
          Version {u.current}. {u.checkedAtMs > 0 && !u.error && "This is the latest."}
        </p>
      )}
      <div className="row small">
        <button type="button" className="ghost" disabled={u.checking || installing} onClick={() => void invoke("check_update")}>
          {u.checking ? "Checking…" : "Check now"}
        </button>
        <span className="muted">
          {u.checkedAtMs > 0 ? `Checked ${relativeTime(u.checkedAtMs, now)}, every day.` : "Checks once a day."}
        </span>
      </div>
      {(error ?? u.error) && <p className={noRelease ? "small muted" : "small error"}>{error ?? u.error}</p>}
      <small className="muted">
        Updates come from <Link href={RELEASES}>GitHub releases</Link> and are checked against the app's signing key before
        they install.
      </small>
    </div>
  );
}
