import { describe, expect, it } from "vitest";
import { displayTitle, isMarkdownPath, lastOpenedText, splitPinned } from "./recent";
import type { RecentDocument } from "../lib/types";

function doc(path: string, extra: Partial<RecentDocument> = {}): RecentDocument {
  return { path, title: null, theme: null, lastOpened: 0, pinned: false, exists: true, ...extra };
}

describe("isMarkdownPath", () => {
  it("accepts Markdown and text files, with either separator", () => {
    expect(isMarkdownPath("C:\\notes\\Draft.MD")).toBe(true);
    expect(isMarkdownPath("/home/a/book.markdown")).toBe(true);
    expect(isMarkdownPath("C:\\notes\\todo.txt")).toBe(true);
  });

  it("refuses other files and folders", () => {
    expect(isMarkdownPath("C:\\pics\\photo.png")).toBe(false);
    expect(isMarkdownPath("C:\\notes")).toBe(false);
    expect(isMarkdownPath("C:\\notes\\.md")).toBe(false);
    expect(isMarkdownPath("C:\\md.folder\\setup.exe")).toBe(false);
  });
});

describe("displayTitle", () => {
  it("prefers the stored title, else the file name without its extension", () => {
    expect(displayTitle(doc("C:\\a\\x.md", { title: "The Long Winter" }))).toBe("The Long Winter");
    expect(displayTitle(doc("C:\\a\\chapter one.md"))).toBe("chapter one");
    expect(displayTitle(doc("/a/README"))).toBe("README");
  });
});

describe("splitPinned", () => {
  it("keeps each group's order", () => {
    const docs = [doc("a", { pinned: true }), doc("b"), doc("c", { pinned: true }), doc("d")];
    const { pinned, recent } = splitPinned(docs);
    expect(pinned.map((d) => d.path)).toEqual(["a", "c"]);
    expect(recent.map((d) => d.path)).toEqual(["b", "d"]);
  });
});

describe("lastOpenedText", () => {
  const now = new Date(2026, 9, 16, 15, 0).getTime();

  it("counts minutes and hours today", () => {
    expect(lastOpenedText(now - 10_000, now)).toBe("just now");
    expect(lastOpenedText(now - 60_000, now)).toBe("1 minute ago");
    expect(lastOpenedText(now - 5 * 60_000, now)).toBe("5 minutes ago");
    expect(lastOpenedText(now - 3 * 3_600_000, now)).toBe("3 hours ago");
  });

  it("counts calendar days after that", () => {
    expect(lastOpenedText(new Date(2026, 9, 15, 9, 0).getTime(), now)).toBe("yesterday");
    expect(lastOpenedText(new Date(2026, 9, 13, 23, 0).getTime(), now)).toBe("3 days ago");
    expect(lastOpenedText(new Date(2026, 8, 1).getTime(), now)).not.toMatch(/ago|yesterday/);
  });
});
