import { useState } from "react";
import { useApp } from "../store";
import type { ApiSnapshot } from "../types";
import { Link } from "./Link";

/** Admin key (kept in the OS keychain) and the credit allowance. */
export function ApiSettingsForm() {
  const api = useApp((s) => s.api);
  if (!api) return <p className="meta">Loading…</p>;
  return <Form key={`${api.settings.allowanceUsd}-${api.settings.resetDay}-${api.keyHint}`} api={api} />;
}

function Form({ api }: { api: ApiSnapshot }) {
  const saveSettings = useApp((s) => s.saveApiSettings);
  const setKey = useApp((s) => s.setAdminKey);
  const [key, setKeyText] = useState("");
  const [allowance, setAllowance] = useState(String(api.settings.allowanceUsd));
  const [resetDay, setResetDay] = useState(String(api.settings.resetDay));
  const [message, setMessage] = useState<{ text: string; error: boolean } | null>(null);

  async function run(action: () => Promise<void>, done: string) {
    setMessage(null);
    try {
      await action();
      setMessage({ text: done, error: false });
    } catch (e) {
      setMessage({ text: String(e), error: true });
    }
  }

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    await run(async () => {
      if (key.trim()) await setKey(key.trim());
      await saveSettings({ allowanceUsd: Number(allowance), resetDay: Number(resetDay) });
      setKeyText("");
    }, "Saved.");
  }

  return (
    <form className="form" onSubmit={(e) => void onSubmit(e)}>
      <label>
        <span>Admin API key</span>
        <input
          type="password"
          autoComplete="off"
          spellCheck={false}
          value={key}
          placeholder={api.configured ? `Saved (${api.keyHint ?? ""}); paste a new one to replace it` : "sk-ant-admin…"}
          onChange={(e) => setKeyText(e.target.value)}
        />
        <small className="meta">
          Create one under <Link href="https://platform.claude.com/settings/admin-keys">Console settings, Admin keys</Link>.
          It is kept in the OS keychain and only sent to api.anthropic.com.
        </small>
      </label>
      <label>
        <span>Monthly credit ($)</span>
        <input type="number" min="1" step="any" value={allowance} onChange={(e) => setAllowance(e.target.value)} />
      </label>
      <label>
        <span>Resets on day of the month (UTC)</span>
        <input type="number" min="1" max="28" step="1" value={resetDay} onChange={(e) => setResetDay(e.target.value)} />
      </label>
      <div className="row">
        <button type="submit">Save</button>
        {api.configured && (
          <button type="button" className="danger" onClick={() => void run(() => setKey(null), "Key removed.")}>
            Remove the Admin key
          </button>
        )}
        {message && <span className={message.error ? "error" : "meta"}>{message.text}</span>}
      </div>
    </form>
  );
}
