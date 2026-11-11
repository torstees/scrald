import { describe, expect, it } from "vitest";
import { conversions, lineEndingLabel, tidySummary } from "./text";

describe("line endings in the status bar", () => {
  it("names the document's line endings", () => {
    expect(lineEndingLabel("lf", false, false)).toBe("LF");
    expect(lineEndingLabel("crlf", false, true)).toBe("CRLF · BOM");
    expect(lineEndingLabel("crlf", true, false)).toBe("Mixed");
  });

  it("offers the other line ending, or both for a mixed file", () => {
    expect(conversions("lf", false)).toEqual(["crlf"]);
    expect(conversions("crlf", false)).toEqual(["lf"]);
    expect(conversions("lf", true)).toEqual(["lf", "crlf"]);
  });
});

describe("tidy summary", () => {
  it("says what changed", () => {
    expect(tidySummary(0, 0)).toBe("No hard-wrapped lines to tidy");
    expect(tidySummary(1, 0)).toBe("Joined 1 line");
    expect(tidySummary(12, 3)).toBe("Joined 12 lines and started 3 new paragraphs");
    expect(tidySummary(0, 1)).toBe("Started 1 new paragraph");
  });
});
