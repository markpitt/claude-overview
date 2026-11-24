import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AlertSettings } from "../types";

const KINDS: { key: keyof Omit<AlertSettings, "thresholdPct">; label: string }[] = [
  { key: "incidents", label: "A Claude incident starts or is resolved" },
  { key: "plan", label: "The 5-hour block or the last 7 days pass the threshold of your allowance" },
  { key: "credit", label: "API spend passes the threshold of the monthly credit" },
  { key: "prs", label: "A pull request needs your review, or Claude opens one" },
];

/** Which desktop notifications to show. Each threshold alerts once, until use drops back below it. */
export function AlertSettingsForm() {
  const [settings, setSettings] = useState<AlertSettings | null>(null);
  const [message, setMessage] = useState<{ text: string; error: boolean } | null>(null);

  useEffect(() => {
    invoke<AlertSettings>("get_alert_settings")
      .then(setSettings)
      .catch((e) => setMessage({ text: String(e), error: true }));
  }, []);

  if (!settings) return <p className="meta">Loading…</p>;

  async function save(next: AlertSettings, done: string) {
    setMessage(null);
    try {
      await invoke("save_alert_settings", { settings: next });
      setSettings(next);
      setMessage({ text: done, error: false });
    } catch (e) {
      setMessage({ text: String(e), error: true });
    }
  }

  async function test() {
    setMessage(null);
    try {
      await invoke("test_alert");
      setMessage({ text: "Sent a test notification.", error: false });
    } catch (e) {
      setMessage({ text: String(e), error: true });
    }
  }

  return (
    <div className="form">
      {KINDS.map((k) => (
        <label key={k.key} className="check">
          <input
            type="checkbox"
            checked={settings[k.key]}
            onChange={(e) => void save({ ...settings, [k.key]: e.target.checked }, "Saved.")}
          />
          {k.label}
        </label>
      ))}
      <label>
        <span>Threshold</span>
        <select
          value={settings.thresholdPct}
          onChange={(e) => void save({ ...settings, thresholdPct: Number(e.target.value) }, "Saved.")}
        >
          {[50, 60, 70, 75, 80, 85, 90, 95, 100].map((p) => (
            <option key={p} value={p}>
              {p}%
            </option>
          ))}
        </select>
        <small className="meta">Plan alerts need your allowance estimates in the section above.</small>
      </label>
      <div className="row">
        <button type="button" onClick={() => void test()}>
          Send a test notification
        </button>
      </div>
      {message && <p className={message.error ? "error" : "meta"}>{message.text}</p>}
    </div>
  );
}
