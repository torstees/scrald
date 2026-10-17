import { describe, expect, it } from "vitest";
import { changeList, colorFieldValue, colorPickerValue, parseList, parseWeights, recordChange, withAlphaOf } from "./editor";

describe("color fields", () => {
  it("accept hex in the usual forms and empty for derived", () => {
    expect(colorFieldValue("#AABBCC")).toBe("#aabbcc");
    expect(colorFieldValue("abc")).toBe("#abc");
    expect(colorFieldValue(" #d6a46344 ")).toBe("#d6a46344");
    expect(colorFieldValue("")).toBeNull();
    expect(colorFieldValue("#12")).toBeUndefined();
    expect(colorFieldValue("blue")).toBeUndefined();
  });

  it("give the picker six digits and keep alpha from the text", () => {
    expect(colorPickerValue("#abc")).toBe("#aabbcc");
    expect(colorPickerValue("#d6a46344")).toBe("#d6a463");
    expect(withAlphaOf("#112233", "#d6a46344")).toBe("#11223344");
    expect(withAlphaOf("#112233", "#abcd")).toBe("#112233dd");
    expect(withAlphaOf("#112233", null)).toBe("#112233");
  });
});

describe("lists", () => {
  it("parse weights and names", () => {
    expect(parseWeights("400, 700 900")).toEqual([400, 700, 900]);
    expect(parseWeights("bold, 0, 1200, 600")).toEqual([600]);
    expect(parseList(" Charter, , Georgia ")).toEqual(["Charter", "Georgia"]);
  });
});

describe("pending changes", () => {
  it("keep the latest value per key, in edit order", () => {
    let changes = recordChange(new Map(), "colors.link", "#111111");
    changes = recordChange(changes, "layout.measure", 70);
    changes = recordChange(changes, "colors.link", "#222222");
    expect(changeList(changes)).toEqual([
      { key: "layout.measure", value: 70 },
      { key: "colors.link", value: "#222222" },
    ]);
  });

  it("removing a font slot drops edits inside it, and editing it again cancels the removal", () => {
    let changes = recordChange(new Map(), "fonts.heading.family", "Cormorant");
    changes = recordChange(changes, "fonts.heading", null);
    expect(changeList(changes)).toEqual([{ key: "fonts.heading", value: null }]);
    changes = recordChange(changes, "fonts.heading.family", "Literata");
    expect(changeList(changes)).toEqual([{ key: "fonts.heading.family", value: "Literata" }]);
  });
});
