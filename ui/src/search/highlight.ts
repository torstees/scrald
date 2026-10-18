// Highlighting search matches in the reader (DESIGN.md §10), with the CSS
// Custom Highlight API: ranges are painted by the browser, so the blocks'
// DOM (keyed by content, DESIGN.md §9.1) is never changed.

import { matchSpans, type SearchOptions } from "./matcher";

/**
 * Content the reader draws itself (math, diagrams, music) and controls it
 * adds; core leaves the same things out of the text it counts
 * (`visible_text` in crates/scrald-core/src/search.rs).
 */
const NOT_TEXT = [
  "[data-math-style]",
  "[data-sk-rendered]",
  "code.language-mermaid",
  "code.language-abc",
  ".sk-mermaid",
  ".sk-abc",
  ".sk-render-error",
  ".sk-backref",
].join(", ");

/** The ranges of each match of `needle` in `root`'s text, in order. */
export function matchRanges(root: Element, needle: string, options: SearchOptions): Range[] {
  const pieces: string[] = [];
  const nodes: Array<Text | null> = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, {
    acceptNode: (node) =>
      node instanceof Element && node.matches(NOT_TEXT) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT,
  });
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (node instanceof Text) {
      pieces.push(node.data);
      nodes.push(node);
    } else if (node instanceof HTMLBRElement) {
      pieces.push(" ");
      nodes.push(null);
    }
  }
  const ranges: Range[] = [];
  for (const span of matchSpans(pieces, needle, options)) {
    const start = nodes[span.startPiece];
    const end = nodes[span.endPiece];
    // A match can't start or end on a <br>: it would be a space at an end of the needle.
    if (!start || !end) continue;
    const range = document.createRange();
    range.setStart(start, span.startOffset);
    range.setEnd(end, span.endOffset);
    ranges.push(range);
  }
  return ranges;
}

const ALL = "sk-search";
const CURRENT = "sk-search-current";

/** Whether this webview supports custom highlights (WebView2 does). */
function supported(): boolean {
  return typeof CSS !== "undefined" && "highlights" in CSS && typeof Highlight !== "undefined";
}

/** Paints `ranges` as matches and `current` as the current one; styled in app.css. */
export function paintMatches(ranges: Range[], current: Range | null): void {
  if (!supported()) return;
  CSS.highlights.set(ALL, new Highlight(...ranges));
  if (current) {
    CSS.highlights.set(CURRENT, new Highlight(current));
  } else {
    CSS.highlights.delete(CURRENT);
  }
}

export function clearMatches(): void {
  if (!supported()) return;
  CSS.highlights.delete(ALL);
  CSS.highlights.delete(CURRENT);
}

/** Opens any collapsed callouts or `<details>` around `node`, so a match inside is visible. */
export function revealNode(node: Node): void {
  for (let el = node.parentElement; el; el = el.parentElement) {
    if (el instanceof HTMLDetailsElement && !el.open) el.open = true;
  }
}
