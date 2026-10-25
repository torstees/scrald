import { describe, expect, it } from "vitest";
import type { FrontSettings, ImageAsset } from "../lib/types";
import { frontContent, frontShown } from "./front";

const image: ImageAsset = { id: 0, path: "C:/docs/cover.png", width: 800, height: 600 };
const both = { frontCover: true, frontTitle: true };
const asTheme: FrontSettings = { cover: "theme", title: "theme" };

describe("frontShown", () => {
  it("follows the theme unless the user chose", () => {
    expect(frontShown("theme", true)).toBe(true);
    expect(frontShown("theme", false)).toBe(false);
    expect(frontShown("show", false)).toBe(true);
    expect(frontShown("hide", true)).toBe(false);
  });
});

describe("frontContent", () => {
  it("shows the cover first, then a missing title", () => {
    expect(frontContent({ cover: image, missingTitle: "Winter" }, both, asTheme)).toEqual({ kind: "cover", image });
    expect(frontContent({ cover: null, missingTitle: "Winter" }, both, asTheme)).toEqual({
      kind: "title",
      text: "Winter",
    });
    expect(frontContent({ cover: null, missingTitle: null }, both, asTheme)).toBeNull();
  });

  it("lets the title take a hidden cover's place", () => {
    const doc = { cover: image, missingTitle: "Winter" };
    expect(frontContent(doc, { frontCover: false, frontTitle: true }, asTheme)?.kind).toBe("title");
    expect(frontContent(doc, both, { cover: "hide", title: "theme" })?.kind).toBe("title");
  });

  it("lets the user override a theme that turns them off", () => {
    const off = { frontCover: false, frontTitle: false };
    expect(frontContent({ cover: null, missingTitle: "Winter" }, off, asTheme)).toBeNull();
    expect(frontContent({ cover: null, missingTitle: "Winter" }, off, { cover: "theme", title: "show" })?.kind).toBe(
      "title",
    );
    expect(frontContent({ cover: image, missingTitle: null }, off, { cover: "show", title: "hide" })?.kind).toBe(
      "cover",
    );
  });
});
