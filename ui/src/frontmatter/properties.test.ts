import { describe, expect, it } from "vitest";
import type { FrontMatter } from "../lib/types";
import { formatValue, isWebLink, propertyRows } from "./properties";

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
  });

  it("is empty without properties", () => {
    expect(propertyRows(frontMatter({}))).toEqual([]);
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
