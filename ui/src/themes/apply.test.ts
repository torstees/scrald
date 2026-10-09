import { describe, expect, it } from "vitest";
import { describeSource, stepIndex } from "./apply";

describe("stepIndex", () => {
  it("wraps in both directions", () => {
    expect(stepIndex(0, 1, 4)).toBe(1);
    expect(stepIndex(3, 1, 4)).toBe(0);
    expect(stepIndex(0, -1, 4)).toBe(3);
    expect(stepIndex(2, 0, 4)).toBe(2);
    expect(stepIndex(0, 1, 0)).toBe(0);
  });
});

describe("describeSource", () => {
  it("explains each source", () => {
    expect(describeSource("document")).toMatch(/this document/);
    expect(describeSource("frontMatter")).toMatch(/front matter/);
    expect(describeSource("folder")).toMatch(/\.scrald\.toml/);
    expect(describeSource("default")).toMatch(/default/);
  });
});
