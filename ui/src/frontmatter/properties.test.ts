import { describe, expect, it } from "vitest";
import type { FrontMatter } from "../lib/types";
import { formatValue, isWebLink, missingProperties, propertyRows } from "./properties";

function frontMatter(overrides: Partial<FrontMatter>): FrontMatter {
  return {
    range: { start: 0, end: 0 },
    title: null,
    authors: [],
    summary: null,
    tags: [],
    notes: null,
    source: null,
    assets: null,
    theme: null,
    flavor: null,
    extra: {},
    keys: {
      title: null,
      authors: null,
      summary: null,
      tags: null,
      notes: null,
      source: null,
      assets: null,
      theme: null,
      flavor: null,
    },
    fields: [],
    error: null,
    ...overrides,
  };
}

describe("propertyRows", () => {
  it("lists recognized properties in order, then other keys", () => {
    const fm = frontMatter({
      title: "The Long Winter",
      tags: ["novel", "norse"],
      authors: ["A. Writer", "B. Writer"],
      extra: { created: "2024-01-02", scrald: { theme: "x" }, list: [1, 2] },
      keys: { ...frontMatter({}).keys, title: "title", tags: "keywords", authors: "authors" },
      fields: [
        { key: "keywords", editable: true, reason: null },
        { key: "title", editable: true, reason: null },
        { key: "authors", editable: true, reason: null },
        { key: "created", editable: true, reason: null },
        { key: "scrald", editable: false, reason: "it's a nested map" },
        { key: "list", editable: true, reason: null },
      ],
    });
    const rows = propertyRows(fm);
    expect(rows.map((r) => [r.label, r.key, r.kind])).toEqual([
      ["Title", "title", "text"],
      ["Authors", "authors", "list"],
      ["Tags", "keywords", "tags"],
      ["created", "created", "value"],
      ["scrald", "scrald", "value"],
      ["list", "list", "list"],
    ]);
    expect(rows[4]).toMatchObject({ value: "theme: x", editable: false, reason: "it's a nested map" });
    expect(rows[5]?.value).toEqual(["1", "2"]);
    expect(rows[0]?.property).toBe("title");
    expect(rows[3]).toMatchObject({ property: null, typed: false });
  });

  it("is empty without properties", () => {
    expect(propertyRows(frontMatter({}))).toEqual([]);
  });
});

describe("missingProperties", () => {
  it("offers recognized properties that aren't set, with flat scrald keys", () => {
    const fm = frontMatter({
      title: "T",
      keys: { ...frontMatter({}).keys, title: "title" },
      fields: [{ key: "title", editable: true, reason: null }],
    });
    const missing = missingProperties(fm);
    expect(missing.map((p) => p.key)).toEqual([
      "author",
      "summary",
      "tags",
      "notes",
      "source",
      "assets",
      "scrald-theme",
      "scrald-flavor",
    ]);
  });

  it("never offers a key that already exists in another shape", () => {
    const fm = frontMatter({ fields: [{ key: "tags", editable: false, reason: "nested" }] });
    expect(missingProperties(fm).some((p) => p.key === "tags")).toBe(false);
  });

  it("offers everything without front matter", () => {
    expect(missingProperties(null)).toHaveLength(9);
  });
});

describe("formatValue", () => {
  it("formats YAML data as short text", () => {
    expect(formatValue(null)).toBe("");
    expect(formatValue([1, "a", true])).toBe("1, a, true");
    expect(formatValue({ a: 1, b: [2, 3] })).toBe("a: 1, b: 2, 3");
  });
});

describe("isWebLink", () => {
  it("accepts only http(s)", () => {
    expect(isWebLink("https://example.com")).toBe(true);
    expect(isWebLink("javascript:alert(1)")).toBe(false);
    expect(isWebLink("file:///etc/passwd")).toBe(false);
  });
});
