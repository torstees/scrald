import { describe, expect, it } from "vitest";
import { hasMod, keyLabel } from "./platform";

describe("hasMod", () => {
  it("is Ctrl elsewhere and Cmd on a Mac", () => {
    expect(hasMod({ ctrlKey: true, metaKey: false }, false)).toBe(true);
    expect(hasMod({ ctrlKey: false, metaKey: true }, false)).toBe(false);
    expect(hasMod({ ctrlKey: false, metaKey: true }, true)).toBe(true);
    expect(hasMod({ ctrlKey: true, metaKey: false }, true)).toBe(false);
  });
});

describe("keyLabel", () => {
  it("leaves shortcuts alone off a Mac", () => {
    expect(keyLabel("Ctrl+Shift+Z", false)).toBe("Ctrl+Shift+Z");
  });

  it("names Cmd and Option on a Mac", () => {
    expect(keyLabel("Ctrl+Shift+\\", true)).toBe("Cmd+Shift+\\");
    expect(keyLabel("Ctrl+Alt+G", true)).toBe("Cmd+Option+G");
    expect(keyLabel("Text size (Ctrl+wheel, Ctrl+0)", true)).toBe("Text size (Cmd+wheel, Cmd+0)");
    expect(keyLabel("F1", true)).toBe("F1");
  });
});
