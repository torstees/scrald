// After the first model of a document, the app sends blocks this window
// already shows without their HTML (DESIGN.md §9.1); `ModelTracker` fills it
// back in from the last model it received.

import type { Block, OpenedDocument } from "./types";

export class ModelTracker {
  private model = 0;
  private blocks: Block[] = [];

  /**
   * Fills in the HTML left out of `opened` and remembers it as the model
   * this window shows. Null if it builds on a model this window doesn't
   * have (then ask for the whole model).
   */
  /** The number of the model this window shows (0 before the first). */
  get current(): number {
    return this.model;
  }

  receive(opened: OpenedDocument): OpenedDocument | null {
    if (opened.base !== null) {
      if (opened.base !== this.model) return null;
      for (const [i, block] of opened.document.blocks.entries()) {
        const from = opened.htmlFrom[i] ?? null;
        if (from === null) continue;
        const shown = this.blocks[from];
        if (!shown) return null;
        block.html = shown.html;
      }
    }
    this.model = opened.model;
    this.blocks = opened.document.blocks;
    return opened;
  }
}
