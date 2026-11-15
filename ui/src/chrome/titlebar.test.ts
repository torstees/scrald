import { describe, expect, it } from "vitest";
import { fitPath, separatorOf } from "./titlebar";

/** Fits text of at most `n` characters. */
const upTo = (n: number) => (text: string) => text.length <= n;

describe("fitPath", () => {
  const path = "~/dev/notes/shopping/tasks.md";

  it("keeps the whole path when it fits", () => {
    expect(fitPath(path, upTo(100))).toBe(path);
  });

  it("drops folders from the start, keeping the file name", () => {
    expect(fitPath(path, upTo(25))).toBe("…/notes/shopping/tasks.md");
    expect(fitPath(path, upTo(19))).toBe("…/shopping/tasks.md");
    expect(fitPath(path, upTo(12))).toBe("…/tasks.md");
    // Never less than the file name.
    expect(fitPath(path, upTo(3))).toBe("…/tasks.md");
  });

  it("follows Windows separators", () => {
    expect(separatorOf("C:\\Users\\ann\\a.md")).toBe("\\");
    expect(fitPath("C:\\Users\\ann\\a.md", upTo(10))).toBe("…\\ann\\a.md");
  });

  it("leaves a bare name alone", () => {
    expect(fitPath("tasks.md", upTo(3))).toBe("tasks.md");
  });
});
