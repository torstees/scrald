import { describe, expect, it } from "vitest";
import type { ContrastProblem } from "../lib/types";
import { colorLabel, describeProblems, formatRatio, problemsFor } from "./contrast";

const muted: ContrastProblem = {
  what: "muted text on the sidebar",
  ratio: 4.158,
  minimum: 4.5,
  colors: ["muted", "ui_background"],
  fg: "#77736a",
  bg: "#f4f0e6",
};
const yellow: ContrastProblem = {
  what: "yellow code on code",
  ratio: 3.4,
  minimum: 4.5,
  colors: ["palette.yellow", "code_bg"],
  fg: "#a1782a",
  bg: "#f1ece0",
};

describe("contrast wording", () => {
  it("names semantic and palette colors", () => {
    expect(colorLabel("ui_background")).toBe("Sidebar and status bar");
    expect(colorLabel("palette.bright_red")).toBe("Palette bright red");
    expect(colorLabel("unknown")).toBe("unknown");
  });

  it("cuts ratios instead of rounding them up to a pass", () => {
    expect(formatRatio(4.158)).toBe("4.15:1");
    expect(formatRatio(4.499)).toBe("4.49:1");
    expect(formatRatio(3)).toBe("3.00:1");
  });

  it("finds the problems for one color", () => {
    expect(problemsFor("code_bg", [muted, yellow])).toEqual([yellow]);
    expect(problemsFor("link", [muted, yellow])).toEqual([]);
  });

  it("describes problems one per line", () => {
    expect(describeProblems([muted, yellow])).toBe(
      "muted text on the sidebar: 4.15:1, needs 4.5:1\nyellow code on code: 3.40:1, needs 4.5:1",
    );
  });
});
