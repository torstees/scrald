import { describe, expect, it } from "vitest";
import { autoLabel, switcherValue } from "./flavor";

describe("flavor switcher", () => {
  it("shows a chosen flavor, otherwise auto", () => {
    expect(switcherValue("pandoc", "document")).toBe("pandoc");
    expect(switcherValue("pandoc", "detected")).toBe("auto");
    expect(switcherValue("gfm", "default")).toBe("auto");
  });

  it("explains what auto resolves to", () => {
    expect(autoLabel("obsidian", "detected")).toBe("Auto: Obsidian (detected)");
    expect(autoLabel("pandoc", "frontMatter")).toBe("Auto: Pandoc (front matter)");
    expect(autoLabel("gfm", "folder")).toBe("Auto: GFM (.scrald.toml)");
    expect(autoLabel("gfm", "default")).toBe("Auto: GFM");
  });
});
