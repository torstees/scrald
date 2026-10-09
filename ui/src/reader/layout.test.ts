import { describe, expect, it } from "vitest";
import type { Block, BlockKind, DocumentModel, Section } from "../lib/types";
import { batchByBlocks, estimateBlockHeight, estimateSectionHeights, mountOrder, sectionBlocks } from "./layout";

function block(id: number, section: number, length: number, kind: BlockKind = { type: "paragraph" }): Block {
  return { id, kind, html: "", source: { start: id * 1000, end: id * 1000 + length }, hash: 0, section };
}

function section(id: number, firstBlock: number, blockCount: number): Section {
  return { id, headingBlock: null, level: 0, firstBlock, blockCount };
}

function doc(blocks: Block[], sections: Section[]): DocumentModel {
  return {
    path: "x.md",
    hasBom: false,
    lineEnding: "lf",
    frontMatter: null,
    blocks,
    sections,
    toc: [],
    wordCount: 0,
    features: { hasMath: false, hasMermaid: false, hasCode: false, hasAbc: false },
    images: [],
    remoteImages: 0,
    remoteImagesAllowed: false,
    flavor: "gfm",
    flavorSource: "default",
    inlineFootnotes: [],
  };
}

describe("mountOrder", () => {
  it("fans out from the start, forward first", () => {
    expect(mountOrder(6, 2)).toEqual([2, 3, 1, 4, 0, 5]);
  });
  it("handles edges and empty documents", () => {
    expect(mountOrder(3, 0)).toEqual([0, 1, 2]);
    expect(mountOrder(3, 2)).toEqual([2, 1, 0]);
    expect(mountOrder(3, 99)).toEqual([2, 1, 0]);
    expect(mountOrder(0, 0)).toEqual([]);
  });
});

describe("batchByBlocks", () => {
  const sections = [section(0, 0, 50), section(1, 50, 50), section(2, 100, 500), section(3, 600, 10)];
  it("groups sections up to the block budget without splitting one", () => {
    expect(batchByBlocks([0, 1, 2, 3], sections, 120)).toEqual([[0, 1], [2], [3]]);
  });
  it("keeps the given order", () => {
    expect(batchByBlocks([3, 0, 1], sections, 100)).toEqual([[3, 0], [1]]);
  });
});

describe("estimates", () => {
  it("longer paragraphs are taller", () => {
    expect(estimateBlockHeight(block(0, 0, 600))).toBeGreaterThan(estimateBlockHeight(block(1, 0, 60)));
  });
  it("sums blocks per section", () => {
    const blocks = [block(0, 0, 60), block(1, 0, 60), block(2, 1, 60)];
    const d = doc(blocks, [section(0, 0, 2), section(1, 2, 1)]);
    const [a, b] = estimateSectionHeights(d);
    expect(a).toBeCloseTo(2 * (b ?? 0), -1);
  });
  it("sectionBlocks slices by section", () => {
    const blocks = [block(0, 0, 1), block(1, 1, 1), block(2, 1, 1)];
    const d = doc(blocks, [section(0, 0, 1), section(1, 1, 2)]);
    expect(sectionBlocks(d, section(1, 1, 2)).map((b) => b.id)).toEqual([1, 2]);
  });
});
