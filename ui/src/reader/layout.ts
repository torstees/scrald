// Pure helpers for the reader's section layout: size estimates and the
// order in which sections are mounted (DESIGN.md §4).

import type { Block, DocumentModel, Section } from "../lib/types";

/** Rough typography numbers used only for estimates; real sizes come from CSS. */
export interface EstimateMetrics {
  /** Characters per line in the text column. */
  charsPerLine: number;
  /** Line height in pixels. */
  lineHeight: number;
  /** Vertical space around each block in pixels. */
  blockGap: number;
}

export const DEFAULT_METRICS: EstimateMetrics = {
  charsPerLine: 66,
  lineHeight: 29,
  blockGap: 18,
};

/** Estimated rendered height of one block, from its source length. */
export function estimateBlockHeight(block: Block, m: EstimateMetrics = DEFAULT_METRICS): number {
  const chars = block.source.end - block.source.start;
  switch (block.kind.type) {
    case "heading":
      return m.lineHeight * 1.8 + m.blockGap * 2;
    case "thematicBreak":
      return m.blockGap * 3;
    case "codeBlock":
    case "table": {
      // Roughly one rendered line per source line.
      const lines = Math.max(1, Math.round(chars / 30));
      return lines * m.lineHeight * 0.9 + m.blockGap * 2;
    }
    default: {
      const lines = Math.max(1, Math.ceil(chars / m.charsPerLine));
      return lines * m.lineHeight + m.blockGap;
    }
  }
}

/** Estimated height of every section, in section order. */
export function estimateSectionHeights(doc: DocumentModel, m: EstimateMetrics = DEFAULT_METRICS): number[] {
  const heights = doc.sections.map(() => 0);
  for (const block of doc.blocks) {
    heights[block.section] = (heights[block.section] ?? 0) + estimateBlockHeight(block, m);
  }
  return heights.map((h) => Math.round(h));
}

/** The blocks belonging to a section. */
export function sectionBlocks(doc: DocumentModel, section: Section): Block[] {
  return doc.blocks.slice(section.firstBlock, section.firstBlock + section.blockCount);
}

/**
 * Section indices ordered outward from `start`: start, start+1, start-1,
 * start+2, ... Sections after the start come first at each distance, since
 * readers usually move forward.
 */
export function mountOrder(count: number, start: number): number[] {
  const order: number[] = [];
  const s = Math.min(Math.max(start, 0), Math.max(count - 1, 0));
  if (count === 0) return order;
  order.push(s);
  for (let d = 1; order.length < count; d++) {
    if (s + d < count) order.push(s + d);
    if (s - d >= 0) order.push(s - d);
  }
  return order;
}

/**
 * Splits a mount order into batches of roughly `blocksPerBatch` blocks, so
 * each idle callback does a bounded amount of DOM work. A section is never
 * split; a huge section gets a batch of its own.
 */
export function batchByBlocks(order: number[], sections: Section[], blocksPerBatch: number): number[][] {
  const batches: number[][] = [];
  let current: number[] = [];
  let size = 0;
  for (const index of order) {
    const count = sections[index]?.blockCount ?? 0;
    if (current.length > 0 && size + count > blocksPerBatch) {
      batches.push(current);
      current = [];
      size = 0;
    }
    current.push(index);
    size += count;
  }
  if (current.length > 0) batches.push(current);
  return batches;
}
