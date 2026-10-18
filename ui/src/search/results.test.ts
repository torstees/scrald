import { describe, expect, it } from "vitest";
import type { SearchReply } from "../lib/types";
import { blocksInRange, firstMatchFrom, locateMatch, matchLabel } from "./results";

const reply: SearchReply = {
  model: 1,
  total: 6,
  blocks: [
    { index: 2, count: 2 },
    { index: 5, count: 1 },
    { index: 9, count: 2 },
  ],
  footnotes: [{ index: 0, count: 1 }],
};

describe("locateMatch", () => {
  it("numbers blocks in order, then footnotes", () => {
    expect(locateMatch(reply, 0)).toEqual({ kind: "block", index: 2, nth: 0 });
    expect(locateMatch(reply, 1)).toEqual({ kind: "block", index: 2, nth: 1 });
    expect(locateMatch(reply, 2)).toEqual({ kind: "block", index: 5, nth: 0 });
    expect(locateMatch(reply, 4)).toEqual({ kind: "block", index: 9, nth: 1 });
    expect(locateMatch(reply, 5)).toEqual({ kind: "footnote", index: 0, nth: 0 });
    expect(locateMatch(reply, 6)).toBeNull();
    expect(locateMatch(reply, -1)).toBeNull();
  });
});

describe("firstMatchFrom", () => {
  it("starts at the reading position", () => {
    expect(firstMatchFrom(reply, 0)).toBe(0);
    expect(firstMatchFrom(reply, 3)).toBe(2);
    expect(firstMatchFrom(reply, 5)).toBe(2);
    // After the last matching block: the endnotes.
    expect(firstMatchFrom(reply, 10)).toBe(5);
    // Without footnote matches, it wraps to the first.
    expect(firstMatchFrom({ ...reply, footnotes: [], total: 5 }, 10)).toBe(0);
    expect(firstMatchFrom({ model: 1, total: 0, blocks: [], footnotes: [] }, 0)).toBe(-1);
  });
});

describe("blocksInRange", () => {
  it("keeps the blocks between two ids", () => {
    expect(blocksInRange(reply, 3, 9).map((h) => h.index)).toEqual([5, 9]);
  });
});

describe("matchLabel", () => {
  it("describes the matches", () => {
    expect(matchLabel(reply, 2, "x")).toBe("3 of 6");
    expect(matchLabel({ ...reply, total: 0 }, -1, "x")).toBe("No matches");
    expect(matchLabel(reply, -1, "x")).toBe("6 matches");
    expect(matchLabel(null, -1, "x")).toBe("");
    expect(matchLabel(reply, 0, "  ")).toBe("");
  });
});
