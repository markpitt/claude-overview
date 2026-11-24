import { useState } from "react";
import { useApp } from "../store";
import { Link } from "./Link";

/** The batches API key, the GitHub token and the Claude Code hooks. */
export function WaitingSettingsForm() {
  const w = useApp((s) => s.waiting);
  const setKey = useApp((s) => s.setWaitingKey);
  const setHooks = useApp((s) => s.setHooks);
  const [apiKey, setApiKey] = useState("");
  const [token, setToken] = useState("");
  const [message, setMessage] = useState<{ text: string; error: boolean } | null>(null);

  if (!w) return <p className="meta">Loading…</p>;

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
      if (apiKey.trim()) await setKey("api", apiKey.trim());
      if (token.trim()) await setKey("github", token.trim());
      setApiKey("");
      setToken("");
    }, "Saved.");
  }

  return (
    <form className="form" onSubmit={(e) => void onSubmit(e)}>
      <label>
        <span>Claude Code hooks</span>
        <span className="row">
          {w.hooks.installed ? "Added to Claude Code's settings." : "Not added."}
          <button
            type="button"
            className={w.hooks.installed ? "danger" : undefined}
            onClick={() => void run(() => setHooks(!w.hooks.installed), w.hooks.installed ? "Hooks removed." : "Hooks added.")}
          >
            {w.hooks.installed ? "Remove the Claude Code hooks" : "Add hooks"}
          </button>
        </span>
        <small className="meta">
          In <code>{w.hooks.settingsPath ?? "~/.claude/settings.json"}</code>, with a backup beside it.
        </small>
      </label>
      <label>
        <span>API key, for batches</span>
        <input
          type="password"
          autoComplete="off"
          spellCheck={false}
          value={apiKey}
          placeholder={w.batches.configured ? `Saved (${w.batches.keyHint ?? ""}); paste a new one to replace it` : "sk-ant-api…"}
          onChange={(e) => setApiKey(e.target.value)}
        />
        <small className="meta">
          An ordinary key from <Link href="https://platform.claude.com/settings/keys">Console settings, API keys</Link>,
          in the workspace that runs your batches.
        </small>
      </label>
      <label>
        <span>GitHub token, for pull requests</span>
        <input
          type="password"
          autoComplete="off"
          spellCheck={false}
          value={token}
          placeholder={w.prs.configured ? `Saved (${w.prs.keyHint ?? ""}); paste a new one to replace it` : "github_pat_…"}
          onChange={(e) => setToken(e.target.value)}
        />
        <small className="meta">
          A <Link href="https://github.com/settings/personal-access-tokens/new">fine-grained token</Link> with read-only
          Pull requests access to the repositories you want watched.
        </small>
      </label>
      <div className="row">
        <button type="submit">Save</button>
        {w.batches.configured && (
          <button type="button" className="danger" onClick={() => void run(() => setKey("api", null), "API key removed.")}>
            Remove the API key
          </button>
        )}
        {w.prs.configured && (
          <button type="button" className="danger" onClick={() => void run(() => setKey("github", null), "Token removed.")}>
            Remove the GitHub token
          </button>
        )}
      </div>
      {message && <p className={message.error ? "error" : "meta"}>{message.text}</p>}
    </form>
  );
}
