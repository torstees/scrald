import { describe, expect, it } from "vitest";
// The user documentation's shortcut page (DESIGN.md §15), as text.
import shortcutsPage from "../../../docs/shortcuts.md?raw";
import { keyParts, opensSettings, opensShortcuts, SHORTCUT_GROUPS, shownKeys, togglesPanel } from "./shortcuts";

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
  const press = (key: string, code: string, ctrlKey = false, altKey = false) => ({
    key,
    code,
    ctrlKey,
    metaKey: false,
    altKey,
  });

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
    expect(opensShortcuts(press("/", "Slash", true), true, false)).toBe(false);
    expect(opensShortcuts(press("F1", "F1"), true, false)).toBe(true);
  });

  it("takes Cmd+/ on a Mac, not Ctrl+/", () => {
    expect(
      opensShortcuts(
        {
          key: "/",
          code: "Slash",
          ctrlKey: false,
          metaKey: true,
          altKey: false,
        },
        false,
        true,
      ),
    ).toBe(true);
    expect(opensShortcuts(press("/", "Slash", true), false, true)).toBe(false);
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
  const key = (
    k: string,
    code: string,
    mods: {
      ctrlKey?: boolean;
      metaKey?: boolean;
      altKey?: boolean;
      shiftKey?: boolean;
    } = {},
  ) => ({
    key: k,
    code,
    ctrlKey: false,
    metaKey: false,
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

  it("opens on Cmd+, on a Mac", () => {
    expect(opensSettings(key(",", "Comma", { metaKey: true }), true)).toBe(true);
    expect(opensSettings(key(",", "Comma", { ctrlKey: true }), true)).toBe(false);
  });
});

describe("togglesPanel", () => {
  const key = (
    k: string,
    code: string,
    mods: {
      ctrlKey?: boolean;
      metaKey?: boolean;
      altKey?: boolean;
      shiftKey?: boolean;
    } = {},
  ) => ({
    key: k,
    code,
    ctrlKey: false,
    metaKey: false,
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

  it("uses Cmd on a Mac", () => {
    expect(togglesPanel(key("\\", "Backslash", { metaKey: true }), true)).toBe("contents");
    expect(togglesPanel(key("\\", "Backslash", { ctrlKey: true }), true)).toBe(null);
  });
});

describe("shownKeys", () => {
  const find = (action: string) => SHORTCUT_GROUPS.flatMap((g) => g.shortcuts).find((s) => s.action === action);

  it("shows the keys as written off a Mac", () => {
    const back = find("Back to the previous document");
    expect(back && shownKeys(back, false)).toEqual(["Alt+Left", "Mouse back button"]);
  });

  it("shows Cmd on a Mac, and the Mac's own keys where they differ", () => {
    const save = find("Save");
    const back = find("Back to the previous document");
    const redo = find("Redo");
    expect(save && shownKeys(save, true)).toEqual(["Cmd+S"]);
    expect(back && shownKeys(back, true)).toEqual(["Cmd+[", "Mouse back button"]);
    expect(redo && shownKeys(redo, true)).toEqual(["Cmd+Shift+Z"]);
  });
});

describe("docs/shortcuts.md", () => {
  // The page lists the same shortcuts as the dialog, one table per group.
  it("lists every shortcut under its group", () => {
    const sections = shortcutsPage.split(/^## /m);
    for (const group of SHORTCUT_GROUPS) {
      const section = sections.find((text) => text.startsWith(`${group.title}\n`));
      expect(section, `a "## ${group.title}" section`).toBeDefined();
      for (const shortcut of group.shortcuts) {
        const keys = shortcut.keys.map((key) => `\`${key}\``).join(" or ");
        expect(section).toContain(`| ${keys} | ${shortcut.action} |`);
      }
    }
  });

  it("names the Mac's own keys", () => {
    for (const shortcut of SHORTCUT_GROUPS.flatMap((g) => g.shortcuts)) {
      for (const keys of shortcut.mac ?? []) {
        if (keys.startsWith("Cmd")) expect(shortcutsPage).toContain(`\`${keys}\``);
      }
    }
  });

  it("lists nothing the dialog doesn't", () => {
    const rows = shortcutsPage.split("\n").filter((line) => line.startsWith("| `"));
    const count = SHORTCUT_GROUPS.reduce((n, group) => n + group.shortcuts.length, 0);
    expect(rows).toHaveLength(count);
  });
});
