// The scroll anchor model (DESIGN.md §4): a reading position stored as a
// source offset plus a fraction into that block, never as pixels, so it
// survives theme changes, zoom, resizes, reloads, and small edits.

import type { Block } from "../lib/types";

export interface ScrollAnchor {
  /** `source.start` of the block at the top of the viewport. */
  offset: number;
  /** How far the viewport top is into that block, 0..1. */
  fraction: number;
}

export const TOP_ANCHOR: ScrollAnchor = { offset: 0, fraction: 0 };

/**
 * Index of the block containing `offset`: the last block that starts at or
 * before it. Offsets before the first block map to block 0. Blocks are in
 * source order (core guarantees this), so a binary search works.
 */
export function blockIndexAtOffset(blocks: Block[], offset: number): number {
  let lo = 0;
  let hi = blocks.length - 1;
  let found = 0;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const block = blocks[mid];
    if (block !== undefined && block.source.start <= offset) {
      found = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return found;
}

/** Fraction of the way `viewTop` is through a box at `top` with `height`. */
export function fractionInto(viewTop: number, top: number, height: number): number {
  if (height <= 0) return 0;
  return Math.min(1, Math.max(0, (viewTop - top) / height));
}

/** A box's vertical extent: its top and its height. */
export interface Extent {
  top: number;
  height: number;
}

/**
 * Index of the first of `count` boxes whose bottom is below `y`. Boxes must
 * be in ascending vertical order; `extentAt(i)` is only called O(log n)
 * times, so it can measure DOM elements directly. Returns the last index if
 * no box reaches `y`, and 0 when there are no boxes.
 */
export function firstBoxBelow(count: number, extentAt: (index: number) => Extent, y: number): number {
  let lo = 0;
  let hi = count - 1;
  let found = count - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const { top, height } = extentAt(mid);
    if (top + height > y) {
      found = mid;
      hi = mid - 1;
    } else {
      lo = mid + 1;
    }
  }
  return Math.max(0, found);
}
