import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Autosaver } from "./autosave";

describe("Autosaver", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("saves once editing pauses", () => {
    const save = vi.fn(() => Promise.resolve());
    const autosaver = new Autosaver(1000, save);
    autosaver.touch();
    vi.advanceTimersByTime(999);
    expect(save).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(save).toHaveBeenCalledTimes(1);
    expect(autosaver.pending).toBe(false);
  });

  it("restarts the wait on every edit", () => {
    const save = vi.fn(() => Promise.resolve());
    const autosaver = new Autosaver(1000, save);
    for (let i = 0; i < 5; i++) {
      autosaver.touch();
      vi.advanceTimersByTime(800);
    }
    expect(save).not.toHaveBeenCalled();
    vi.advanceTimersByTime(200);
    expect(save).toHaveBeenCalledTimes(1);
  });

  it("can be cancelled", () => {
    const save = vi.fn(() => Promise.resolve());
    const autosaver = new Autosaver(1000, save);
    autosaver.touch();
    expect(autosaver.pending).toBe(true);
    autosaver.cancel();
    vi.advanceTimersByTime(5000);
    expect(save).not.toHaveBeenCalled();
  });
});
