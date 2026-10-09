// Pure helpers for the table of contents sidebar.

import type { TocEntry } from "../lib/types";

/** Whether the entry at `index` has deeper entries directly under it. */
export function hasChildren(toc: TocEntry[], index: number): boolean {
  const entry = toc[index];
  const next = toc[index + 1];
  return entry !== undefined && next !== undefined && next.level > entry.level;
}

/**
 * Indices of entries to show: at or above `maxLevel`, and not inside a
 * collapsed entry. `collapsed` holds indices of collapsed entries.
 */
export function visibleEntries(toc: TocEntry[], maxLevel: number, collapsed: ReadonlySet<number>): number[] {
  const visible: number[] = [];
  // Level of the collapsed ancestor we're inside, or Infinity when none.
  let hiddenBelow = Infinity;
  toc.forEach((entry, index) => {
    if (entry.level <= hiddenBelow) hiddenBelow = Infinity;
    if (entry.level > hiddenBelow) return;
    if (entry.level > maxLevel) return;
    visible.push(index);
    if (collapsed.has(index)) hiddenBelow = entry.level;
  });
  return visible;
}

/** Index of the TOC entry for the heading that starts `blockId`'s section. */
export function entryForHeadingBlock(toc: TocEntry[], headingBlock: number | null): number | null {
  if (headingBlock === null) return null;
  const index = toc.findIndex((e) => e.blockId === headingBlock);
  return index >= 0 ? index : null;
}

/**
 * The entry to highlight when `index` is current but may be hidden (by
 * `maxLevel` or a collapsed parent): the nearest visible entry at or before it.
 */
export function nearestVisible(visible: number[], index: number | null): number | null {
  if (index === null) return null;
  let best: number | null = null;
  for (const v of visible) {
    if (v <= index) best = v;
    else break;
  }
  return best;
}
