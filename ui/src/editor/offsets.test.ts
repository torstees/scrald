import { describe, expect, it } from "vitest";
import { byteLength, byteToIndex, indexToByte, indexToEditorPos } from "./offsets";

const TEXT = "a\r\nÅse 🌊 Þór\n";

describe("byte offsets and string indices", () => {
  it("agree with TextEncoder at every character boundary", () => {
    const encoder = new TextEncoder();
    for (let i = 0; i <= TEXT.length; i++) {
      // Skip the middle of the surrogate pair.
      if (i > 0 && TEXT.charCodeAt(i - 1) >= 0xd800 && TEXT.charCodeAt(i - 1) <= 0xdbff) continue;
      const bytes = encoder.encode(TEXT.slice(0, i)).length;
      expect(indexToByte(TEXT, i)).toBe(bytes);
      expect(byteToIndex(TEXT, bytes)).toBe(i);
    }
  });

  it("rounds offsets inside a character back to its start", () => {
    // "Å" is 2 bytes starting at byte 3 (after "a\r\n").
    expect(byteToIndex(TEXT, 4)).toBe(3);
    // The wave emoji is 4 bytes; its index is 7.
    const waveByte = indexToByte(TEXT, 7);
    expect(byteToIndex(TEXT, waveByte + 2)).toBe(7);
  });

  it("clamps past the end", () => {
    expect(byteToIndex(TEXT, 10_000)).toBe(TEXT.length);
    expect(indexToByte(TEXT, 10_000)).toBe(new TextEncoder().encode(TEXT).length);
  });
});

describe("editor positions", () => {
  it("count each CRLF as one position", () => {
    const text = "a\r\nbc\r\nd";
    expect(indexToEditorPos(text, text.indexOf("d"), "\r\n")).toBe(5);
    expect(indexToEditorPos(text, text.indexOf("b"), "\r\n")).toBe(2);
    expect(indexToEditorPos(text, 0, "\r\n")).toBe(0);
    expect(indexToEditorPos("a\nb", 2, "\n")).toBe(2);
  });

  it("measure UTF-8 lengths", () => {
    expect(byteLength("Åse\r\n")).toBe(6);
  });
});
