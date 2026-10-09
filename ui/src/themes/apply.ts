// Applies a theme to the page (DESIGN.md §7.2): its CSS goes into one
// <style> element at the end of <head>, so it overrides the fallback tokens
// in app.css. Switching themes replaces that element's text.

import type { ThemeStyle } from "../lib/types";

const STYLE_ID = "sk-theme";

export function applyTheme(style: ThemeStyle, doc: Document = document): void {
  let element = doc.getElementById(STYLE_ID);
  if (!(element instanceof HTMLStyleElement)) {
    element = doc.createElement("style");
    element.id = STYLE_ID;
    doc.head.appendChild(element);
  }
  element.textContent = style.css;
  doc.documentElement.dataset.theme = style.id;
  doc.documentElement.dataset.appearance = style.appearance;
}

/** Human-readable explanation of where a document's theme came from. */
export function describeSource(source: string): string {
  switch (source) {
    case "document":
      return "chosen for this document";
    case "frontMatter":
      return "from the document's front matter (scrald-theme)";
    case "folder":
      return "from a .scrald.toml in this folder or above";
    default:
      return "the default theme";
  }
}

/** Index after moving `delta` steps through `count` items, wrapping around. */
export function stepIndex(index: number, delta: number, count: number): number {
  if (count === 0) return 0;
  return (((index + delta) % count) + count) % count;
}
