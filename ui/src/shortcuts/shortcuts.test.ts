import { describe, expect, it } from "vitest";
import { keyParts, opensSettings, opensShortcuts, SHORTCUT_GROUPS, togglesPanel } from "./shortcuts";

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

describe("opensSettings", () => {
  const key = (k: string, code: string, mods: { ctrlKey?: boolean; altKey?: boolean; shiftKey?: boolean } = {}) => ({
    key: k,
    code,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...mods,
  });

  it("opens on Ctrl+, (and the physical comma key)", () => {
    expect(opensSettings(key(",", "Comma", { ctrlKey: true }))).toBe(true);
    expect(opensSettings(key(";", "Comma", { ctrlKey: true }))).toBe(true);
  });

  it("leaves a plain comma and other combinations alone", () => {
    expect(opensSettings(key(",", "Comma"))).toBe(false);
    expect(opensSettings(key("<", "Comma", { ctrlKey: true, shiftKey: true }))).toBe(false);
    expect(opensSettings(key(",", "Comma", { ctrlKey: true, altKey: true }))).toBe(false);
  });
});

describe("togglesPanel", () => {
  const key = (k: string, code: string, mods: { ctrlKey?: boolean; altKey?: boolean; shiftKey?: boolean } = {}) => ({
    key: k,
    code,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...mods,
  });

  it("toggles Contents on Ctrl+\ and properties on Ctrl+Shift+\\", () => {
    expect(togglesPanel(key("\\", "Backslash", { ctrlKey: true }))).toBe("contents");
    expect(togglesPanel(key("|", "Backslash", { ctrlKey: true, shiftKey: true }))).toBe("properties");
  });

  it("matches the physical key on other layouts", () => {
    expect(togglesPanel(key("#", "Backslash", { ctrlKey: true }))).toBe("contents");
  });

  it("ignores other keys and Alt", () => {
    expect(togglesPanel(key("\\", "Backslash"))).toBe(null);
    expect(togglesPanel(key("\\", "Backslash", { ctrlKey: true, altKey: true }))).toBe(null);
    expect(togglesPanel(key("p", "KeyP", { ctrlKey: true, shiftKey: true }))).toBe(null);
  });
});
