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

/**
 * Index of the entry for the heading at or above the reading position, when
 * block `blockId` is at the top: the last heading in an earlier block, or
 * in that block itself if `passed(index)` says it has reached the top (a
 * heading inside a fenced div or HTML container may still be further down).
 */
export function currentEntryAt(toc: TocEntry[], blockId: number, passed: (index: number) => boolean): number | null {
  let current: number | null = null;
  for (const [index, entry] of toc.entries()) {
    if (entry.blockId > blockId) break;
    // Headings in one block are in order, so the first not reached ends it.
    if (entry.blockId === blockId && !passed(index)) break;
    current = index;
  }
  return current;
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
