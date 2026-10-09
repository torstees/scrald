import { describe, expect, it } from "vitest";
import type { Block } from "../lib/types";
import { blockIndexAtOffset, firstBoxBelow, fractionInto } from "./anchor";

function blockAt(id: number, start: number, end: number): Block {
  return { id, kind: { type: "paragraph" }, html: "", source: { start, end }, hash: 0, section: 0 };
}

describe("blockIndexAtOffset", () => {
  const blocks = [blockAt(0, 10, 20), blockAt(1, 22, 40), blockAt(2, 42, 50)];
  it("finds the block containing the offset", () => {
    expect(blockIndexAtOffset(blocks, 10)).toBe(0);
    expect(blockIndexAtOffset(blocks, 30)).toBe(1);
    expect(blockIndexAtOffset(blocks, 42)).toBe(2);
  });
  it("maps gaps to the preceding block and clamps at the ends", () => {
    expect(blockIndexAtOffset(blocks, 21)).toBe(0);
    expect(blockIndexAtOffset(blocks, 0)).toBe(0);
    expect(blockIndexAtOffset(blocks, 9999)).toBe(2);
  });
  it("survives an edit that shifts later blocks", () => {
    // After inserting 5 bytes in block 0, block 1 now starts at 27.
    const edited = [blockAt(0, 10, 25), blockAt(1, 27, 45), blockAt(2, 47, 55)];
    expect(blockIndexAtOffset(edited, 22)).toBe(0);
  });
  it("handles an empty document", () => {
    expect(blockIndexAtOffset([], 5)).toBe(0);
  });
});

describe("fractionInto", () => {
  it("is clamped to 0..1", () => {
    expect(fractionInto(150, 100, 100)).toBe(0.5);
    expect(fractionInto(50, 100, 100)).toBe(0);
    expect(fractionInto(500, 100, 100)).toBe(1);
    expect(fractionInto(100, 100, 0)).toBe(0);
  });
});

describe("firstBoxBelow", () => {
  const boxes = [
    { top: 0, height: 100 },
    { top: 100, height: 150 },
    { top: 250, height: 50 },
    { top: 300, height: 400 },
  ];
  const at = (i: number) => boxes[i] ?? { top: 0, height: 0 };
  it("finds the box under a y position", () => {
    expect(firstBoxBelow(4, at, 0)).toBe(0);
    expect(firstBoxBelow(4, at, 99)).toBe(0);
    expect(firstBoxBelow(4, at, 100)).toBe(1);
    expect(firstBoxBelow(4, at, 260)).toBe(2);
    expect(firstBoxBelow(4, at, 9999)).toBe(3);
  });
  it("handles no boxes", () => {
    expect(firstBoxBelow(0, at, 10)).toBe(0);
  });
});
