import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { HotkeyStatus } from "../types";
import { displayShortcut, fromKeyEvent, isMac } from "../lib/hotkey";

const DEFAULT = "CommandOrControl+Alt+O";

/** A global shortcut that opens the pop-over from anywhere. */
export function HotkeyForm() {
  const [status, setStatus] = useState<HotkeyStatus | null>(null);
  const [recording, setRecording] = useState(false);
  const [message, setMessage] = useState<{ text: string; error: boolean } | null>(null);
  const mac = isMac();

  useEffect(() => {
    invoke<HotkeyStatus>("get_hotkey")
      .then(setStatus)
      .catch((e) => setMessage({ text: String(e), error: true }));
  }, []);

  if (!status) return <p className="meta">Loading…</p>;

  async function save(accelerator: string) {
    setMessage(null);
    setRecording(false);
    try {
      const next = await invoke<HotkeyStatus>("set_hotkey", { accelerator });
      setStatus(next);
      setMessage({ text: accelerator ? "Saved." : "Shortcut turned off.", error: false });
    } catch (e) {
      setMessage({ text: String(e), error: true });
    }
  }

  function onKeyDown(e: React.KeyboardEvent) {
    e.preventDefault();
    if (e.code === "Escape") {
      setRecording(false);
      return;
    }
    const accel = fromKeyEvent(e, mac);
    if (accel) void save(accel);
  }

  return (
    <div className="form">
      <label>
        <span>Open the pop-over</span>
        <span className="row">
          <button
            type="button"
            className={recording ? "shortcut recording" : "shortcut"}
            onClick={() => setRecording(true)}
            onKeyDown={recording ? onKeyDown : undefined}
            onBlur={() => setRecording(false)}
          >
            {recording ? "Press the keys…" : displayShortcut(status.accelerator, mac)}
          </button>
          {status.accelerator !== DEFAULT && (
            <button type="button" onClick={() => void save(DEFAULT)}>
              Use {displayShortcut(DEFAULT, mac)}
            </button>
          )}
          {status.accelerator && (
            <button type="button" onClick={() => void save("")}>
              Turn off
            </button>
          )}
        </span>
        <small className="meta">
          Click, then press the shortcut you want, with at least one of Ctrl, Alt or Shift. It works in any app. On Linux
          under Wayland, bind <code>claude-overview --popover</code> to a key in your desktop's keyboard settings instead.
        </small>
      </label>
      {status.error && <p className="error">{status.error}</p>}
      {message && <p className={message.error ? "error" : "meta"}>{message.text}</p>}
    </div>
  );
}
