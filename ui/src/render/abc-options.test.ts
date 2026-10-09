import { describe, expect, it } from "vitest";
import { filterOptions, parseAbcBlock } from "./abc-options";

const TUNE = 'X:1\nT:The Abbey\nK:Ador\n|"Am"A3B A2GE|A2GA BddB|';

describe("parseAbcBlock", () => {
  it("passes a plain tune through", () => {
    expect(parseAbcBlock(TUNE)).toEqual({ tune: TUNE, options: {}, error: null });
  });

  it("passes a headerless snippet through", () => {
    const snippet = '|"Am"A3B A2GE|A2GA BddB|';
    expect(parseAbcBlock(snippet).tune).toBe(snippet);
  });

  it("reads an Obsidian-style JSON header", () => {
    const block = parseAbcBlock(`{"tablature": [{"instrument": "violin"}]}\n---\n${TUNE}`);
    expect(block.options).toEqual({ tablature: [{ instrument: "violin" }] });
    expect(block.tune).toBe(TUNE);
    expect(block.error).toBeNull();
  });

  it("still renders the tune when the JSON is invalid", () => {
    const block = parseAbcBlock(`{"scale": 1.2,}\n---\n${TUNE}`);
    expect(block.tune).toBe(TUNE);
    expect(block.options).toEqual({});
    expect(block.error).toMatch(/not valid JSON/);
  });

  it("reports a header without a separator", () => {
    const block = parseAbcBlock(`{"scale": 1.2}\n${TUNE}`);
    expect(block.error).toMatch(/---/);
  });

  it("handles CRLF", () => {
    const block = parseAbcBlock(`{"scale": 2}\r\n---\r\n${TUNE}`);
    expect(block.options).toEqual({ scale: 2 });
  });
});

describe("filterOptions", () => {
  it("keeps allowlisted options of the right type", () => {
    const { options, ignored } = filterOptions({
      scale: 1.5,
      staffwidth: 600,
      jazzchords: true,
      wrap: { minSpacing: 1.8, maxSpacing: 2.7, preferredMeasuresPerLine: 4 },
    });
    expect(Object.keys(options)).toEqual(["scale", "staffwidth", "jazzchords", "wrap"]);
    expect(ignored).toEqual([]);
  });

  it("drops unknown keys, wrong types, and unknown instruments", () => {
    const { options, ignored } = filterOptions({
      clickListener: "x",
      scale: "big",
      tablature: [{ instrument: "bagpipes" }],
    });
    expect(options).toEqual({});
    expect(ignored).toEqual(["clickListener", "scale", "tablature"]);
  });
});
