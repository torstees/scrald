import { describe, expect, it } from "vitest";
import { launchMessage } from "./launch";

describe("launchMessage", () => {
  it("prompts for a file when launched without one", () => {
    expect(launchMessage({ path: null, title: "Scrald", exists: false })).toMatch(/No document open/);
  });

  it("reports a missing file", () => {
    expect(launchMessage({ path: "/tmp/nope.md", title: "nope", exists: false })).toBe(
      "File not found: /tmp/nope.md",
    );
  });

  it("shows the opened path", () => {
    expect(launchMessage({ path: "/docs/skål.md", title: "skål", exists: true })).toBe(
      "Opened /docs/skål.md",
    );
  });
});
