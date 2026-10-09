import { describe, expect, it } from "vitest";
import { formatReadingTime, formatWordCount } from "./reading";

describe("formatWordCount", () => {
  it("groups thousands", () => {
    expect(formatWordCount(0)).toBe("0 words");
    expect(formatWordCount(1)).toBe("1 word");
    expect(formatWordCount(1234)).toBe("1,234 words");
    expect(formatWordCount(1234567)).toBe("1,234,567 words");
  });
});

describe("formatReadingTime", () => {
  it("formats minutes and hours", () => {
    expect(formatReadingTime(50)).toBe("< 1 min");
    expect(formatReadingTime(238 * 12)).toBe("12 min");
    expect(formatReadingTime(100_000)).toBe("7 h 00 min");
    expect(formatReadingTime(238 * 65)).toBe("1 h 05 min");
  });
});
