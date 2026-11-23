/** The parts of a keydown event a shortcut is built from. */
export interface KeyInput {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
  /** KeyboardEvent.code, such as "KeyO" or "F5": the physical key, whatever the layout. */
  code: string;
}

const MODIFIER_CODES = /^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/;

/** Turns a key press into the app's shortcut form, or null while only modifiers are held. */
export function fromKeyEvent(e: KeyInput, mac: boolean): string | null {
  if (MODIFIER_CODES.test(e.code)) return null;
  const key = keyName(e.code);
  if (!key) return null;
  const parts: string[] = [];
  // Ctrl on Windows and Linux, Cmd on macOS: one shortcut works on both.
  if (mac ? e.metaKey : e.ctrlKey) parts.push("CommandOrControl");
  if (mac && e.ctrlKey) parts.push("Control");
  if (!mac && e.metaKey) parts.push("Super");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (parts.length === 0) return null;
  return [...parts, key].join("+");
}

function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  const named: Record<string, string> = {
    Space: "Space",
    Enter: "Enter",
    Tab: "Tab",
    Backquote: "`",
    Minus: "-",
    Equal: "=",
    BracketLeft: "[",
    BracketRight: "]",
    Backslash: "\\",
    Semicolon: ";",
    Quote: "'",
    Comma: ",",
    Period: ".",
    Slash: "/",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    Insert: "Insert",
    Delete: "Delete",
  };
  return named[code] ?? null;
}

/** "CommandOrControl+Alt+O" as people see it: "Ctrl+Alt+O", or "⌘⌥O" on a Mac. */
export function displayShortcut(accel: string, mac: boolean): string {
  if (!accel) return "Off";
  const parts = accel.split("+");
  const names: Record<string, [string, string]> = {
    CommandOrControl: ["Ctrl", "⌘"],
    CmdOrCtrl: ["Ctrl", "⌘"],
    Control: ["Ctrl", "⌃"],
    Ctrl: ["Ctrl", "⌃"],
    Super: ["Super", "⌘"],
    Alt: ["Alt", "⌥"],
    Option: ["Alt", "⌥"],
    Shift: ["Shift", "⇧"],
  };
  const shown = parts.map((p) => names[p]?.[mac ? 1 : 0] ?? p);
  return mac ? shown.join("") : shown.join("+");
}

export function isMac(): boolean {
  return typeof navigator !== "undefined" && /Mac/.test(navigator.userAgent);
}
