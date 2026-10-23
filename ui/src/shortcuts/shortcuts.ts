// Every keyboard shortcut, grouped as the reference dialog shows them (F1).
// Keep this in step with the handlers: the window's in `App.svelte`, and
// each panel's own (`SearchBar`, `BlockEditor`, `ThemeSwitcher`, ...).
import type { Panel } from "../sidebar/layout";

export interface Shortcut {
  /** Alternatives, each written like "Ctrl+Shift+Z". */
  keys: string[];
  action: string;
}

export interface ShortcutGroup {
  title: string;
  /** Where the group applies, if not everywhere. */
  where?: string;
  shortcuts: Shortcut[];
}

export const SHORTCUT_GROUPS: ShortcutGroup[] = [
  {
    title: "Documents",
    shortcuts: [
      { keys: ["Ctrl+O"], action: "Open a document" },
      { keys: ["Ctrl+N"], action: "Create a new document" },
      { keys: ["Ctrl+S"], action: "Save" },
      { keys: ["Ctrl+P"], action: "Print or save as PDF" },
      { keys: ["Alt+Left", "Mouse back button"], action: "Back to the previous document" },
      { keys: ["Alt+Right", "Mouse forward button"], action: "Forward" },
      { keys: ["F1", "Ctrl+/"], action: "Show these shortcuts" },
      { keys: ["Ctrl+,"], action: "Settings" },
    ],
  },
  {
    title: "Reading",
    shortcuts: [
      { keys: ["Ctrl+\\"], action: "Show or hide the contents" },
      { keys: ["Ctrl+Shift+\\"], action: "Show or hide the properties" },
      { keys: ["Ctrl+T"], action: "Choose a theme" },
      { keys: ["Ctrl+=", "Ctrl+wheel up"], action: "Zoom in" },
      { keys: ["Ctrl+-", "Ctrl+wheel down"], action: "Zoom out" },
      { keys: ["Ctrl+0"], action: "Reset zoom" },
    ],
  },
  {
    title: "Find",
    shortcuts: [
      { keys: ["Ctrl+F"], action: "Find in the document" },
      { keys: ["Enter", "F3"], action: "Next match" },
      { keys: ["Shift+Enter", "Shift+F3"], action: "Previous match" },
      { keys: ["Esc"], action: "Close the find bar" },
    ],
  },
  {
    title: "Editing",
    shortcuts: [
      { keys: ["Double-click"], action: "Edit a block" },
      { keys: ["Ctrl+Enter", "Esc"], action: "Finish editing the block" },
      { keys: ["Ctrl+E"], action: "Switch to raw mode and back" },
      { keys: ["Ctrl+Z"], action: "Undo" },
      { keys: ["Ctrl+Y", "Ctrl+Shift+Z"], action: "Redo" },
    ],
  },
  {
    title: "Raw mode",
    shortcuts: [
      { keys: ["Ctrl+F"], action: "Find and replace" },
      { keys: ["F3", "Ctrl+G"], action: "Next match" },
      { keys: ["Shift+F3", "Ctrl+Shift+G"], action: "Previous match" },
      { keys: ["Ctrl+Alt+G"], action: "Go to line" },
      { keys: ["Ctrl+/"], action: "Comment lines in or out" },
    ],
  },
  {
    title: "Properties",
    where: "while editing a value",
    shortcuts: [
      { keys: ["Enter"], action: "Keep the change" },
      { keys: ["Ctrl+Enter"], action: "Keep the change to a summary or notes" },
      { keys: ["Esc"], action: "Cancel" },
    ],
  },
  {
    title: "Themes",
    shortcuts: [
      { keys: ["Up", "Down"], action: "Preview the previous or next theme" },
      { keys: ["Enter"], action: "Use the theme for this document" },
      { keys: ["Ctrl+Enter"], action: "Save the theme being edited" },
      { keys: ["Esc"], action: "Put the theme back" },
    ],
  },
];

/**
 * The keys of one shortcut, to show one per box: "Ctrl+Shift+Z" is
 * ["Ctrl", "Shift", "Z"], and "Ctrl++" is ["Ctrl", "+"].
 */
export function keyParts(keys: string): string[] {
  return keys.split(/\+(?!$)/);
}

/**
 * The panel a key press shows or hides: Ctrl+\ for Contents, Ctrl+Shift+\
 * for properties. The physical key counts too: on many non-US layouts the
 * backslash isn't on it.
 */
export function togglesPanel(event: Pick<KeyboardEvent, "key" | "code" | "ctrlKey" | "altKey" | "shiftKey">): Panel | null {
  if (!event.ctrlKey || event.altKey) return null;
  if (event.key !== "\\" && event.key !== "|" && event.code !== "Backslash") return null;
  return event.shiftKey ? "properties" : "contents";
}

/** Whether a key press opens the settings dialog: Ctrl+, (the physical key too). */
export function opensSettings(event: Pick<KeyboardEvent, "key" | "code" | "ctrlKey" | "altKey" | "shiftKey">): boolean {
  return event.ctrlKey && !event.altKey && !event.shiftKey && (event.key === "," || event.code === "Comma");
}

/**
 * Whether a key press opens the shortcut reference: F1, or Ctrl+/ (the
 * physical key too). In an editor, Ctrl+/ is left to CodeMirror, which uses
 * it to comment lines in or out.
 */
export function opensShortcuts(
  event: Pick<KeyboardEvent, "key" | "code" | "ctrlKey" | "altKey">,
  inCodeEditor: boolean,
): boolean {
  if (event.key === "F1") return true;
  return !inCodeEditor && event.ctrlKey && !event.altKey && (event.key === "/" || event.code === "Slash");
}
