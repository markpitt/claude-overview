import { useEffect, useState } from "react";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { getVersion } from "@tauri-apps/api/app";
import { Link } from "../components/Link";
import { CcSettingsForm } from "../components/CcSettingsForm";
import { ApiSettingsForm } from "../components/ApiSettingsForm";
import { WaitingSettingsForm } from "../components/WaitingSettingsForm";
import { AlertSettingsForm } from "../components/AlertSettingsForm";
import { HotkeyForm } from "../components/HotkeyForm";
import { UpdatesPanel } from "../components/UpdatesPanel";
import { CardHead } from "../components/CardHead";

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
        <CardHead title="General" />
        <label className="check">
          <input
            type="checkbox"
            checked={autostart ?? false}
            disabled={autostart === null}
            onChange={(e) => void toggle(e.target.checked)}
          />
          Start at login, in the tray
        </label>
        {error && <p className="error">{error}</p>}
      </section>
      <section className="card">
        <CardHead title="Max plan and Claude Code" />
        <CcSettingsForm />
      </section>
      <section className="card">
        <CardHead title="API credit" />
        <ApiSettingsForm />
      </section>
      <section className="card">
        <CardHead title="What's waiting" />
        <WaitingSettingsForm />
        <p className="meta note">Keys and tokens are kept in the OS keychain, never on disk.</p>
      </section>
      <section className="card">
        <CardHead title="Alerts" />
        <AlertSettingsForm />
      </section>
      <section className="card">
        <CardHead title="Keyboard shortcut" />
        <HotkeyForm />
      </section>
      <section className="card">
        <CardHead title="Updates" />
        <UpdatesPanel />
      </section>
      <section className="card">
        <CardHead title="About" />
        <p>
          Claude Overview {version && `v${version}`}. Source and releases at{" "}
          <Link href="https://github.com/markpitt/claude-overview">github.com/markpitt/claude-overview</Link>.
        </p>
      </section>
    </div>
  );
}
