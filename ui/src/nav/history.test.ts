import { describe, expect, it } from "vitest";
import { NavHistory } from "./history";

const at = (offset: number) => ({ offset, fraction: 0 });

describe("NavHistory", () => {
  it("goes back and forward, remembering positions", () => {
    const h = new NavHistory();
    h.push({ path: "a.md", anchor: at(0) });
    h.updateAnchor(at(500));
    h.push({ path: "b.md", anchor: at(0) });
    expect(h.canGoBack).toBe(true);
    expect(h.canGoForward).toBe(false);

    h.updateAnchor(at(90));
    expect(h.back()).toEqual({ path: "a.md", anchor: at(500) });
    expect(h.canGoForward).toBe(true);
    expect(h.forward()).toEqual({ path: "b.md", anchor: at(90) });
  });

  it("drops forward entries when a new document is opened", () => {
    const h = new NavHistory();
    h.push({ path: "a.md", anchor: at(0) });
    h.push({ path: "b.md", anchor: at(0) });
    h.back();
    h.push({ path: "c.md", anchor: at(0) });
    expect(h.canGoForward).toBe(false);
    expect(h.back()?.path).toBe("a.md");
  });

  it("is safe at the ends", () => {
    const h = new NavHistory();
    expect(h.back()).toBeNull();
    expect(h.forward()).toBeNull();
    expect(h.current).toBeNull();
    h.updateAnchor(at(1));
  });
});
