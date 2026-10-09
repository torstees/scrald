import { describe, expect, it } from "vitest";
import type { TocEntry } from "../lib/types";
import { entryForHeadingBlock, hasChildren, nearestVisible, visibleEntries } from "./tree";

function e(level: number, blockId: number): TocEntry {
  return { level, text: `h${blockId}`, slug: `h${blockId}`, blockId, number: null };
}

// 0: H1, 1: H2, 2: H3, 3: H2, 4: H1, 5: H2
const toc = [e(1, 0), e(2, 5), e(3, 9), e(2, 12), e(1, 20), e(2, 25)];

describe("toc tree", () => {
  it("hasChildren", () => {
    expect([0, 1, 2, 3, 4, 5].map((i) => hasChildren(toc, i))).toEqual([true, true, false, false, true, false]);
  });

  it("shows everything by default", () => {
    expect(visibleEntries(toc, 6, new Set())).toEqual([0, 1, 2, 3, 4, 5]);
  });

  it("limits by level", () => {
    expect(visibleEntries(toc, 1, new Set())).toEqual([0, 4]);
    expect(visibleEntries(toc, 2, new Set())).toEqual([0, 1, 3, 4, 5]);
  });

  it("hides children of collapsed entries", () => {
    expect(visibleEntries(toc, 6, new Set([0]))).toEqual([0, 4, 5]);
    expect(visibleEntries(toc, 6, new Set([1]))).toEqual([0, 1, 3, 4, 5]);
  });

  it("finds the entry for a section heading", () => {
    expect(entryForHeadingBlock(toc, 12)).toBe(3);
    expect(entryForHeadingBlock(toc, null)).toBeNull();
    expect(entryForHeadingBlock(toc, 999)).toBeNull();
  });

  it("highlights the nearest visible ancestor when the current entry is hidden", () => {
    const visible = visibleEntries(toc, 1, new Set());
    expect(nearestVisible(visible, 2)).toBe(0);
    expect(nearestVisible(visible, 5)).toBe(4);
    expect(nearestVisible(visible, null)).toBeNull();
  });
});
