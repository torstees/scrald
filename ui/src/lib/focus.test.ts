import { describe, expect, it } from "vitest";
import { nextFocusIndex } from "./focus";

describe("nextFocusIndex", () => {
  it("moves forward and wraps to the first", () => {
    expect(nextFocusIndex(3, 0, false)).toBe(1);
    expect(nextFocusIndex(3, 2, false)).toBe(0);
  });

  it("moves backward and wraps to the last", () => {
    expect(nextFocusIndex(3, 1, true)).toBe(0);
    expect(nextFocusIndex(3, 0, true)).toBe(2);
  });

  it("enters at either end when focus is outside", () => {
    expect(nextFocusIndex(3, -1, false)).toBe(0);
    expect(nextFocusIndex(3, -1, true)).toBe(2);
  });

  it("has nowhere to go without controls", () => {
    expect(nextFocusIndex(0, -1, false)).toBe(-1);
  });
});
