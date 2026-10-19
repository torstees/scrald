import { describe, expect, it } from "vitest";
import { chooseTab, isShown, layout, toggle, type SidebarState } from "./layout";

const both: SidebarState = { leftOpen: true, propertiesOpen: true, last: "contents" };
const none: SidebarState = { leftOpen: false, propertiesOpen: false, last: "contents" };

describe("sidebar layout", () => {
  it("keeps properties out of the sidebars at the top", () => {
    expect(layout("top", both, true)).toEqual({ left: "contents", right: false });
    expect(isShown("properties", "top", both, true)).toBe(true);
    expect(toggle("properties", "top", both, true)).toEqual({ ...both, propertiesOpen: false });
  });

  it("shows one tab of the left sidebar", () => {
    expect(layout("left", both, true)).toEqual({ left: "properties", right: false });
    expect(layout("left", { ...both, propertiesOpen: false }, true)).toEqual({ left: "contents", right: false });
    expect(layout("left", none, true)).toEqual({ left: null, right: false });
  });

  it("switches tabs before hiding the left sidebar", () => {
    // On the Properties tab, Ctrl+\ shows Contents rather than hiding.
    const contents = toggle("contents", "left", both, true);
    expect(layout("left", contents, true).left).toBe("contents");
    expect(layout("left", toggle("contents", "left", contents, true), true).left).toBe(null);
    // Hidden on the Properties tab, it comes back there.
    const hidden = toggle("properties", "left", both, true);
    expect(hidden.leftOpen).toBe(false);
    expect(layout("left", toggle("properties", "left", hidden, true), true).left).toBe("properties");
    expect(chooseTab("contents")).toEqual({ leftOpen: true, propertiesOpen: false, last: "contents" });
  });

  it("shows both sidebars in a wide window", () => {
    expect(layout("right", both, true)).toEqual({ left: "contents", right: true });
  });

  it("shows only the panel opened last in a narrow window", () => {
    expect(layout("right", both, false)).toEqual({ left: "contents", right: false });
    expect(layout("right", { ...both, last: "properties" }, false)).toEqual({ left: null, right: true });
  });

  it("brings a collapsed panel back instead of hiding it", () => {
    // Narrow: Contents opened last, so properties are collapsed though open.
    expect(isShown("properties", "right", both, false)).toBe(false);
    const next = toggle("properties", "right", both, false);
    expect(next).toEqual({ ...both, last: "properties" });
    expect(layout("right", next, false)).toEqual({ left: null, right: true });
    // And Ctrl+\ brings Contents back the same way.
    expect(layout("right", toggle("contents", "right", next, false), false)).toEqual({ left: "contents", right: false });
  });

  it("hides the right sidebar when it's shown", () => {
    expect(toggle("properties", "right", both, true)).toEqual({ ...both, propertiesOpen: false });
  });
});
