import { describe, expect, it } from "vitest";
import { formatProgress, formatTimeLeft, readingProgress, wordsBefore } from "./progress";

const blocks = [{ words: 10 }, { words: 0 }, { words: 30 }, { words: 60 }];
const before = wordsBefore(blocks);

describe("wordsBefore", () => {
  it("adds up the words before each block, then the total", () => {
    expect(before).toEqual([0, 10, 10, 40, 100]);
    expect(wordsBefore([])).toEqual([0]);
  });
});

describe("readingProgress", () => {
  it("counts the words above the top of the screen", () => {
    expect(readingProgress(before, { blockIndex: 0, fraction: 0, atEnd: false })).toBe(0);
    expect(readingProgress(before, { blockIndex: 2, fraction: 0, atEnd: false })).toBe(0.1);
    expect(readingProgress(before, { blockIndex: 3, fraction: 0.5, atEnd: false })).toBe(0.7);
  });

  it("is complete once the end is on screen", () => {
    expect(readingProgress(before, { blockIndex: 1, fraction: 0, atEnd: true })).toBe(1);
  });

  it("copes with a document without words and with indexes past the end", () => {
    expect(readingProgress([0], { blockIndex: 0, fraction: 0.5, atEnd: false })).toBe(0);
    expect(readingProgress(before, { blockIndex: 9, fraction: 0, atEnd: false })).toBe(1);
    expect(readingProgress(before, { blockIndex: 3, fraction: 7, atEnd: false })).toBe(1);
  });
});

describe("formatProgress", () => {
  it("rounds down to whole percent", () => {
    expect(formatProgress(0)).toBe("0%");
    expect(formatProgress(0.999)).toBe("99%");
    expect(formatProgress(1)).toBe("100%");
  });
});

describe("formatTimeLeft", () => {
  it("shows the total at the start and the end, the time left between", () => {
    expect(formatTimeLeft(238 * 12, 0)).toBe("12 min");
    expect(formatTimeLeft(238 * 12, 0.5)).toBe("6 min left");
    expect(formatTimeLeft(238 * 12, 0.99)).toBe("< 1 min left");
    expect(formatTimeLeft(238 * 12, 1)).toBe("12 min");
  });
});
