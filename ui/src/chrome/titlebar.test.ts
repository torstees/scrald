import { describe, expect, it } from "vitest";
import { fitPath, fittedTitle, separatorOf } from "./titlebar";

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

describe("fittedTitle", () => {
  const title = { title: "Groceries", dirty: true, path: "C:\\Users\\ann\\notes\\groceries.md" };

  it("keeps the whole title when it fits", () => {
    expect(fittedTitle(title, upTo(100))).toBe("• Groceries — C:\\Users\\ann\\notes\\groceries.md");
  });

  it("shortens the path from the start to fit the whole text", () => {
    // "• Groceries — " is 14 characters.
    expect(fittedTitle(title, upTo(14 + 20))).toBe("• Groceries — …\\notes\\groceries.md");
  });

  it("is just the title without a path", () => {
    expect(fittedTitle({ title: "Scrald", dirty: false, path: null }, upTo(3))).toBe("Scrald");
  });
});
