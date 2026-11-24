import { useState } from "react";
import { useApp } from "../store";
import type { CcSettings } from "../types";

function toText(v: number | null | undefined): string {
  return v ? String(v) : "";
}

function toNumber(s: string): number | null {
  const n = Number(s.trim());
  return s.trim() && Number.isFinite(n) && n > 0 ? n : null;
}

/** Folder override and your own Max plan estimates. Saved in the local database. */
export function CcSettingsForm() {
  const cc = useApp((s) => s.cc);
  const save = useApp((s) => s.saveCcSettings);
  if (!cc) return <p className="meta">Loading…</p>;
  // Remount when the saved settings change so the fields show them.
  return <Form key={JSON.stringify(cc.settings)} initial={cc.settings} defaultFolder={cc.folder} save={save} />;
}

function Form({
  initial,
  defaultFolder,
  save,
}: {
  initial: CcSettings;
  defaultFolder: string | null;
  save: (s: CcSettings) => Promise<void>;
}) {
  const [folder, setFolder] = useState(initial.folderOverride ?? "");
  const [block, setBlock] = useState(toText(initial.blockLimitUsd));
  const [week, setWeek] = useState(toText(initial.weekLimitUsd));
  const [plan, setPlan] = useState(toText(initial.planPriceUsd));
  const [message, setMessage] = useState<string | null>(null);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setMessage(null);
    try {
      await save({
        folderOverride: folder.trim() || null,
        blockLimitUsd: toNumber(block),
        weekLimitUsd: toNumber(week),
        planPriceUsd: toNumber(plan),
      });
      setMessage("Saved.");
    } catch (err) {
      setMessage(String(err));
    }
  }

  return (
    <form className="form" onSubmit={(e) => void onSubmit(e)}>
      <label>
        <span>Claude Code folder</span>
        <input
          type="text"
          value={folder}
          placeholder={initial.folderOverride ? "" : (defaultFolder ?? "~/.claude/projects")}
          onChange={(e) => setFolder(e.target.value)}
        />
        <small className="meta">Leave empty for the default. The folder that holds one folder per project.</small>
      </label>
      <label>
        <span>5-hour block allowance ($ at API prices)</span>
        <input type="number" min="0" step="any" value={block} onChange={(e) => setBlock(e.target.value)} />
      </label>
      <label>
        <span>Weekly allowance ($ at API prices)</span>
        <input type="number" min="0" step="any" value={week} onChange={(e) => setWeek(e.target.value)} />
      </label>
      <label>
        <span>Plan price per month ($)</span>
        <input type="number" min="0" step="any" value={plan} onChange={(e) => setPlan(e.target.value)} />
      </label>
      <p className="meta">
        Anthropic doesn't publish Max plan limits. Note what a block cost when you last hit a limit and use that as
        your allowance.
      </p>
      <div className="row">
        <button type="submit">Save</button>
        {message && <span className="meta">{message}</span>}
      </div>
    </form>
  );
}
