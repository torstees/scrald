import { describe, expect, it } from "vitest";
import { colorDeclarations, cssString, defaultPrintOptions, linkShowsAddress, printCss } from "./print";

describe("cssString", () => {
  it("quotes and escapes", () => {
    expect(cssString("The Long Winter")).toBe('"The Long Winter"');
    expect(cssString('Say "skål" \\ cheers')).toBe('"Say \\"skål\\" \\\\ cheers"');
  });

  it("escapes control characters so a title can't end the rule", () => {
    expect(cssString("a\nb")).toBe('"a\\a b"');
    expect(cssString('"; } body { color: red')).toBe('"\\"; } body { color: red"');
  });
});

describe("colorDeclarations", () => {
  const css = `:root {
  color-scheme: light;
  --sk-color-background: #fbf8f1;
  --sk-color-code-bg: #f1ece0;
  --sk-palette-bright_red: #c4533f;
  --sk-font-body: "Literata", serif;
  --sk-measure: 66;
}
.sk-block hr { color: red; }`;

  it("keeps the colors and leaves fonts and layout", () => {
    expect(colorDeclarations(css)).toEqual([
      "color-scheme: light",
      "--sk-color-background: #fbf8f1",
      "--sk-color-code-bg: #f1ece0",
      "--sk-palette-bright_red: #c4533f",
    ]);
  });

  it("finds nothing without a :root rule", () => {
    expect(colorDeclarations(".x { color: red; }")).toEqual([]);
  });
});

describe("printCss", () => {
  it("puts the title and page numbers in the margins", () => {
    const css = printCss("Saga", { colors: "theme", linkAddresses: false }, []);
    expect(css).toContain('@top-center { content: "Saga";');
    expect(css).toContain('counter(page) " / " counter(pages)');
    expect(css).toContain('@page :first { @top-center { content: ""; } }');
    expect(css).not.toContain(":root");
    expect(css).not.toContain("attr(href)");
  });

  it("uses the paper colors only when asked", () => {
    const colors = ["color-scheme: light", "--sk-color-link: #2f5f8a"];
    expect(printCss("x", { colors: "paper", linkAddresses: false }, colors)).toContain(
      ":root { color-scheme: light; --sk-color-link: #2f5f8a; --sk-color-background: #ffffff; }",
    );
    expect(printCss("x", { colors: "theme", linkAddresses: false }, colors)).not.toContain(":root");
  });

  it("adds link addresses when asked", () => {
    expect(printCss("x", { colors: "theme", linkAddresses: true }, [])).toContain('content: "(" attr(href) ")"');
  });
});

describe("defaultPrintOptions", () => {
  it("prints dark themes on white paper", () => {
    expect(defaultPrintOptions("dark").colors).toBe("paper");
    expect(defaultPrintOptions("light").colors).toBe("theme");
    expect(defaultPrintOptions(undefined)).toEqual({ colors: "theme", linkAddresses: false });
  });
});

describe("linkShowsAddress", () => {
  it("ignores the scheme and a trailing slash", () => {
    expect(linkShowsAddress("example.com", "https://example.com/")).toBe(true);
    expect(linkShowsAddress("https://example.com/a", "https://example.com/a")).toBe(true);
    expect(linkShowsAddress("me@example.com", "mailto:me@example.com")).toBe(true);
    expect(linkShowsAddress("the guide", "https://example.com/guide")).toBe(false);
  });
});
