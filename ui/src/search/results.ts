// Numbering the matches core found (`SearchReply`): blocks in order, then
// footnotes, which the reader shows as endnotes after the last block.

import type { SearchHits, SearchReply } from "../lib/types";

/** One match: the nth (from 0) in a block or a footnote. */
export interface MatchLocation {
  kind: "block" | "footnote";
  /** Block id, or index in `DocumentModel.footnotes`. */
  index: number;
  nth: number;
}

/** Match number `i` (from 0), or null if there's no such match. */
export function locateMatch(results: SearchReply, i: number): MatchLocation | null {
  if (i < 0) return null;
  let rest = i;
  const find = (hits: SearchHits[], kind: MatchLocation["kind"]): MatchLocation | null => {
    for (const hit of hits) {
      if (rest < hit.count) return { kind, index: hit.index, nth: rest };
      rest -= hit.count;
    }
    return null;
  };
  return find(results.blocks, "block") ?? find(results.footnotes, "footnote");
}

/** The number of the first match in block `blockId` or after it (wrapping to the first). */
export function firstMatchFrom(results: SearchReply, blockId: number): number {
  if (results.total === 0) return -1;
  let before = 0;
  for (const hit of results.blocks) {
    if (hit.index >= blockId) return before;
    before += hit.count;
  }
  // Past the last block with a match: the endnotes come next, if they match.
  return results.footnotes.length > 0 ? before : 0;
}

/** The blocks from id `first` to `last` that have matches. */
export function blocksInRange(results: SearchReply, first: number, last: number): SearchHits[] {
  return results.blocks.filter((hit) => hit.index >= first && hit.index <= last);
}

/** "3 of 120", "No matches", or "" for an empty query. */
export function matchLabel(results: SearchReply | null, current: number, query: string): string {
  if (query.trim() === "" || results === null) return "";
  if (results.total === 0) return "No matches";
  if (current < 0) return `${results.total.toLocaleString()} matches`;
  return `${(current + 1).toLocaleString()} of ${results.total.toLocaleString()}`;
}
