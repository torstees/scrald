// The reader's link to block editing (DESIGN.md §9.1); App provides it.
import type { Block } from "../lib/types";

export interface BlockEditing {
  /** The block's Markdown source, from the document text in memory. */
  load: (block: Block) => Promise<string>;
  /** Replaces the block's source; `original` is what the editor started from. */
  commit: (block: Block, original: string, text: string) => Promise<void>;
}
