import { describe, expect, it } from "vitest";
import type { Block } from "../lib/types";
import { formatTasks, taskCounts } from "./tasks";

function block(id: number, tasks?: boolean[]): Block {
  return {
    id,
    kind: { type: "list", ordered: false },
    html: "",
    source: { start: 0, end: 0 },
    hash: id,
    section: 0,
    words: 0,
    tasks: tasks?.map((checked, i) => ({ offset: id * 100 + i, checked })),
  };
}

describe("taskCounts", () => {
  it("counts every block's tasks", () => {
    expect(taskCounts([block(0, [true, false]), block(1), block(2, [true])])).toEqual({ done: 2, total: 3 });
    expect(taskCounts([block(0)])).toEqual({ done: 0, total: 0 });
  });
});

describe("formatTasks", () => {
  it("says how many are done", () => {
    expect(formatTasks(3, 12)).toBe("3 / 12 done");
  });
});
