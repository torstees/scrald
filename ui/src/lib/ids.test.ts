import { describe, expect, it } from "vitest";
import { contentId } from "./ids";

describe("contentId", () => {
  it("adds the prefix once", () => {
    expect(contentId("intro")).toBe("user-content-intro");
    expect(contentId("user-content-fn-1")).toBe("user-content-fn-1");
  });
});
