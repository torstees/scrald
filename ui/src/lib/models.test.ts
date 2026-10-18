import { describe, expect, it } from "vitest";
import { ModelTracker } from "./models";
import type { Block, OpenedDocument } from "./types";

function block(id: number, html: string): Block {
  return { id, kind: { type: "paragraph" }, html, source: { start: id, end: id + 1 }, hash: id, section: 0, words: 0 };
}

function model(n: number, base: number | null, blocks: Block[], htmlFrom: (number | null)[] = []): OpenedDocument {
  // Only the fields the tracker reads matter here.
  return { model: n, base, htmlFrom, document: { blocks } } as unknown as OpenedDocument;
}

describe("ModelTracker", () => {
  it("fills in left-out HTML from the model it shows", () => {
    const tracker = new ModelTracker();
    tracker.receive(model(1, null, [block(0, "<p>a</p>"), block(1, "<p>b</p>")]));
    // Block 0 changed; block 1 is the old block 1, sent without its HTML.
    const next = tracker.receive(model(2, 1, [block(0, "<p>A</p>"), block(1, "")], [null, 1]));
    expect(next?.document.blocks.map((b) => b.html)).toEqual(["<p>A</p>", "<p>b</p>"]);
    // And the next model builds on that one.
    const third = tracker.receive(model(3, 2, [block(0, ""), block(1, "")], [0, 1]));
    expect(third?.document.blocks.map((b) => b.html)).toEqual(["<p>A</p>", "<p>b</p>"]);
  });

  it("refuses a model built on one it doesn't show", () => {
    const tracker = new ModelTracker();
    expect(tracker.receive(model(5, 4, [block(0, "")], [0]))).toBeNull();
    tracker.receive(model(6, null, [block(0, "<p>a</p>")]));
    expect(tracker.receive(model(8, 7, [block(0, "")], [0]))).toBeNull();
    // A block id the shown model doesn't have.
    expect(tracker.receive(model(9, 6, [block(0, "")], [3]))).toBeNull();
  });

  it("takes a whole model at any time", () => {
    const tracker = new ModelTracker();
    tracker.receive(model(1, null, [block(0, "<p>a</p>")]));
    const whole = tracker.receive(model(7, null, [block(0, "<p>z</p>")]));
    expect(whole?.document.blocks[0]?.html).toBe("<p>z</p>");
    expect(tracker.receive(model(8, 7, [block(0, "")], [0]))?.document.blocks[0]?.html).toBe("<p>z</p>");
  });
});
