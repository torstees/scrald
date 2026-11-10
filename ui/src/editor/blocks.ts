// The reader's link to block editing (DESIGN.md §9.1); App provides it.
import type { SourceRange } from "../lib/types";

/** What's being edited: a block, or one of its parts (`within` is the block's range). */
export interface EditTarget {
  range: SourceRange;
  within?: SourceRange;
}

export interface BlockEditing {
  /** The source in `range`, from the document text in memory. */
  load: (range: SourceRange) => Promise<string>;
  /** Replaces the target's source; `original` is what the editor started from. */
  commit: (target: EditTarget, original: string, text: string) => Promise<void>;
}
