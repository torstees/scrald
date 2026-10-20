import { describe, expect, it } from "vitest";
import type { FolderChoice, TemplateChoice } from "../lib/types";
import { asSentence, fileNameForTitle, folderName, localDate, PLAIN, templateFor, UNTITLED } from "./newdoc";

describe("fileNameForTitle", () => {
  it("drops characters file names can't hold", () => {
    expect(fileNameForTitle("Notes: day 1?")).toBe("Notes day 1.md");
    expect(fileNameForTitle('a/b\\c|d*e"f<g>h')).toBe("abcdefgh.md");
  });

  it("collapses spaces and trims trailing dots", () => {
    expect(fileNameForTitle("  The   Long\tWinter...  ")).toBe("The Long Winter.md");
  });

  it("falls back to Untitled", () => {
    expect(fileNameForTitle("")).toBe(UNTITLED);
    expect(fileNameForTitle("???")).toBe(UNTITLED);
  });
});

describe("localDate", () => {
  it("pads months and days", () => {
    expect(localDate(new Date(2026, 0, 5))).toBe("2026-01-05");
  });
});

describe("templateFor", () => {
  const templates: TemplateChoice[] = [
    { id: PLAIN, name: "Plain (GFM)" },
    { id: "builtin:obsidian", name: "Obsidian note" },
  ];
  const folder = (remembered: boolean, template: string | null): FolderChoice => ({
    path: "C:\\Notes",
    remembered,
    template,
    exists: true,
  });

  it("uses a remembered folder's last template", () => {
    expect(templateFor(folder(true, "builtin:obsidian"), templates)).toBe("builtin:obsidian");
  });

  it("starts other folders, and templates that are gone, with Plain", () => {
    expect(templateFor(folder(false, "builtin:obsidian"), templates)).toBe(PLAIN);
    expect(templateFor(folder(true, "user:Gone.md"), templates)).toBe(PLAIN);
    expect(templateFor(undefined, templates)).toBe(PLAIN);
  });
});

describe("folderName", () => {
  it("is the last part of the path", () => {
    expect(folderName("C:\\Users\\me\\Notes\\")).toBe("Notes");
    expect(folderName("/home/me/notes")).toBe("notes");
  });
});

describe("asSentence", () => {
  it("capitalizes the first letter only", () => {
    expect(asSentence("there's already a file named a.md")).toBe("There's already a file named a.md");
    expect(asSentence("")).toBe("");
  });
});
