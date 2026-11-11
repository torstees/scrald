import { describe, expect, it } from "vitest";
import { partAt, partElements, type PartTree } from "./parts";

/** A small stand-in for a DOM element. */
function el(tagName: string, ...children: PartTree[]): PartTree {
  const node: PartTree = {
    tagName,
    children,
    firstElementChild: children[0] ?? null,
    contains: (other) => (other as unknown) === node || children.some((c) => c.contains(other)),
  };
  return node;
}

const asNode = (tree: PartTree) => tree as unknown as Node;

describe("partElements", () => {
  it("finds a list's items and a quote's children", () => {
    const items = [el("LI"), el("LI", el("UL", el("LI")))];
    expect(partElements(el("DIV", el("UL", ...items)))).toEqual(items);
    const children = [el("P"), el("PRE")];
    expect(partElements(el("DIV", el("BLOCKQUOTE", ...children)))).toEqual(children);
    expect(partElements(el("DIV", el("P")))).toEqual([]);
    expect(partElements(el("DIV"))).toEqual([]);
  });
});

describe("partAt", () => {
  const inner = el("EM");
  const nested = el("LI");
  const first = el("LI");
  const items = [first, el("LI", el("P", inner)), el("LI", el("UL", nested))];

  it("finds the part holding the click, nested lists included", () => {
    expect(partAt(items, 3, asNode(inner))).toBe(1);
    expect(partAt(items, 3, asNode(nested))).toBe(2);
    expect(partAt(items, 3, asNode(first))).toBe(0);
  });

  it("edits the whole block when parts and HTML don't match", () => {
    expect(partAt(items, 2, asNode(inner))).toBeNull();
    expect(partAt(items, 0, asNode(inner))).toBeNull();
    expect(partAt(items, 3, asNode(el("SPAN")))).toBeNull();
  });
});
