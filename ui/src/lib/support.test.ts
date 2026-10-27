import { describe, expect, it } from "vitest";
import { FEATURES, forcedOff } from "./support";

describe("forcedOff", () => {
  it("forces nothing off by default", () => {
    expect(forcedOff(undefined).size).toBe(0);
    expect(forcedOff("").size).toBe(0);
  });

  it("reads a comma list, ignoring unknown names", () => {
    expect([...forcedOff(" scrollAnchoring, customHighlights ,nonsense")]).toEqual([
      "scrollAnchoring",
      "customHighlights",
    ]);
  });

  it("forces everything off with all", () => {
    expect([...forcedOff("all")]).toEqual([...FEATURES]);
  });
});
