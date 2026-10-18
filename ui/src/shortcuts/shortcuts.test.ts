import { describe, expect, it } from "vitest";
import { keyParts, opensShortcuts, SHORTCUT_GROUPS } from "./shortcuts";

describe("keyParts", () => {
  it("splits a combination into its keys", () => {
    expect(keyParts("Ctrl+Shift+Z")).toEqual(["Ctrl", "Shift", "Z"]);
    expect(keyParts("F1")).toEqual(["F1"]);
  });

  it("keeps a plus key or a backslash", () => {
    expect(keyParts("Ctrl++")).toEqual(["Ctrl", "+"]);
    expect(keyParts("Ctrl+\\")).toEqual(["Ctrl", "\\"]);
    expect(keyParts("Ctrl+-")).toEqual(["Ctrl", "-"]);
  });
});

describe("opensShortcuts", () => {
  const press = (key: string, code: string, ctrlKey = false, altKey = false) => ({ key, code, ctrlKey, altKey });

  it("opens on F1 and Ctrl+/", () => {
    expect(opensShortcuts(press("F1", "F1"), false)).toBe(true);
    expect(opensShortcuts(press("/", "Slash", true), false)).toBe(true);
  });

  it("matches the physical key on layouts where it isn't a slash", () => {
    expect(opensShortcuts(press("-", "Slash", true), false)).toBe(true);
  });

  it("leaves a plain slash and other keys alone", () => {
    expect(opensShortcuts(press("/", "Slash"), false)).toBe(false);
    expect(opensShortcuts(press("/", "Slash", true, true), false)).toBe(false);
    expect(opensShortcuts(press("f", "KeyF", true), false)).toBe(false);
  });

  it("leaves Ctrl+/ to an editor, but not F1", () => {
    expect(opensShortcuts(press("/", "Slash", true), true)).toBe(false);
    expect(opensShortcuts(press("F1", "F1"), true)).toBe(true);
  });
});

describe("SHORTCUT_GROUPS", () => {
  it("has no empty groups and no repeated action within a group", () => {
    for (const group of SHORTCUT_GROUPS) {
      expect(group.shortcuts.length).toBeGreaterThan(0);
      const actions = group.shortcuts.map((s) => s.action);
      expect(new Set(actions).size).toBe(actions.length);
      for (const shortcut of group.shortcuts) expect(shortcut.keys.length).toBeGreaterThan(0);
    }
  });
});
