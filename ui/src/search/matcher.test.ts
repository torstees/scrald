import { describe, expect, it } from "vitest";
import { findMatches, matchSpans, normalizeQuery, textOf } from "./matcher";

const plain = { matchCase: false, wholeWord: false };

describe("textOf", () => {
  it("collapses whitespace across pieces, like core", () => {
    const map = textOf([" a \n\t b", "\u{a0}c "], false);
    expect(map.text).toBe(" a b c ");
    // The space between "b" and "c" comes from the second piece's first unit.
    expect([map.piece[4], map.offset[4]]).toEqual([1, 0]);
    expect([map.piece[5], map.offset[5]]).toEqual([1, 1]);
  });

  it("lowercases one character at a time, keeping ones that would grow", () => {
    expect(textOf(["ÅSA Straße İ"], true).text).toBe("åsa straße İ");
  });

  it("keeps offsets for characters outside the BMP", () => {
    const map = textOf(["a😀b"], false);
    expect(map.text).toBe("a😀b");
    expect(map.offset).toEqual([0, 1, 2, 3]);
  });
});

describe("findMatches", () => {
  // The same cases as `counting_matches` in crates/scrald-core/src/search.rs.
  const count = (text: string, needle: string, wholeWord: boolean) => findMatches(text, needle, wholeWord).length;

  it("counts like core", () => {
    expect(count("aaaa", "aa", false)).toBe(2);
    expect(count("the theme then", "the", false)).toBe(3);
    expect(count("the theme then", "the", true)).toBe(1);
    expect(count("thethe the", "the", true)).toBe(1);
    expect(count("skål, skål!", "skål", true)).toBe(2);
    expect(count("x", "", false)).toBe(0);
  });

  it("treats digits as word characters", () => {
    expect(count("v2 2 x2", "2", true)).toBe(1);
  });
});

describe("matchSpans", () => {
  it("maps a match across formatting back to its text nodes", () => {
    // <p><strong>Ragnar</strong> Lothbrok</p>
    const spans = matchSpans(["Ragnar", " Lothbrok"], normalizeQuery("ragnar  lothbrok", plain), plain);
    expect(spans).toEqual([{ startPiece: 0, startOffset: 0, endPiece: 1, endOffset: 9 }]);
  });

  it("matches a line break as a space", () => {
    const spans = matchSpans(["one\ntwo"], "one two", plain);
    expect(spans).toEqual([{ startPiece: 0, startOffset: 0, endPiece: 0, endOffset: 7 }]);
  });

  it("respects match case", () => {
    const options = { matchCase: true, wholeWord: false };
    expect(matchSpans(["Theme the THE"], normalizeQuery("THE", options), options)).toHaveLength(1);
  });
});

describe("normalizeQuery", () => {
  it("trims and collapses", () => {
    expect(normalizeQuery("  Two   Words ", plain)).toBe("two words");
    expect(normalizeQuery("   ", plain)).toBe("");
  });
});
