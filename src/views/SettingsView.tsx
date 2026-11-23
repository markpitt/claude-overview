import { useEffect, useState } from "react";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { getVersion } from "@tauri-apps/api/app";
import { Link } from "../components/Link";
import { CcSettingsForm } from "../components/CcSettingsForm";
import { ApiSettingsForm } from "../components/ApiSettingsForm";
import { WaitingSettingsForm } from "../components/WaitingSettingsForm";

export function SettingsView() {
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [version, setVersion] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    isEnabled().then(setAutostart).catch((e) => setError(String(e)));
    getVersion().then(setVersion).catch(() => {});
  }, []);

  async function toggle(on: boolean) {
    setError(null);
    try {
      await (on ? enable() : disable());
      setAutostart(await isEnabled());
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="stack">
      <section className="card">
        <h2>General</h2>
        <label className="row">
          <input
            type="checkbox"
            checked={autostart ?? false}
            disabled={autostart === null}
            onChange={(e) => void toggle(e.target.checked)}
          />
          Start at login, in the tray
        </label>
        {error && <p className="error small">{error}</p>}
      </section>
      <section className="card">
        <h2>Max plan and Claude Code</h2>
        <CcSettingsForm />
      </section>
      <section className="card">
        <h2>API credit</h2>
        <ApiSettingsForm />
      </section>
      <section className="card">
        <h2>What's waiting</h2>
        <WaitingSettingsForm />
        <p className="muted small">Keys and tokens are kept in the OS keychain, never on disk.</p>
      </section>
      <section className="card">
        <h2>About</h2>
        <p className="small">
          Claude Overview {version && `v${version}`}. Source and releases at{" "}
          <Link href="https://github.com/markpitt/claude-overview">github.com/markpitt/claude-overview</Link>.
        </p>
      </section>
    </div>
  );
}
