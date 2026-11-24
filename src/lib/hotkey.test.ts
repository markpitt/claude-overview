import { describe, expect, it } from "vitest";
import { displayShortcut, fromKeyEvent } from "./hotkey";

const key = (code: string, mods: Partial<{ ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean }> = {}) => ({
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  code,
  ...mods,
});

describe("shortcuts", () => {
  it("builds a shortcut from a key press", () => {
    expect(fromKeyEvent(key("KeyO", { ctrlKey: true, altKey: true }), false)).toBe("CommandOrControl+Alt+O");
    expect(fromKeyEvent(key("KeyO", { metaKey: true, altKey: true }), true)).toBe("CommandOrControl+Alt+O");
    expect(fromKeyEvent(key("Space", { altKey: true, shiftKey: true }), false)).toBe("Alt+Shift+Space");
    expect(fromKeyEvent(key("F5", { metaKey: true }), false)).toBe("Super+F5");
  });

  it("waits for a real key and a modifier", () => {
    expect(fromKeyEvent(key("ControlLeft", { ctrlKey: true }), false)).toBeNull();
    expect(fromKeyEvent(key("KeyO"), false)).toBeNull();
    expect(fromKeyEvent(key("IntlBackslash", { ctrlKey: true }), false)).toBeNull();
  });

  it("shows shortcuts the way each OS writes them", () => {
    expect(displayShortcut("CommandOrControl+Alt+O", false)).toBe("Ctrl+Alt+O");
    expect(displayShortcut("CommandOrControl+Alt+O", true)).toBe("⌘⌥O");
    expect(displayShortcut("", false)).toBe("Off");
  });
});
