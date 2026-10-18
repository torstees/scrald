// Finding search matches in a block's text (DESIGN.md §10). Core counts the
// matches in each block (`crates/scrald-core/src/search.rs`); the reader
// finds the same matches in the block's DOM text to highlight them, so this
// reads and matches text exactly the way core does:
// - every run of whitespace counts as one space, and `<br>` as whitespace;
// - without "match case", each character is lowercased on its own, and kept
//   as it is if its lowercase form is more than one character;
// - matches don't overlap, searching from left to right;
// - "whole word" means no letter or digit right before or after a match.

export interface SearchOptions {
  matchCase: boolean;
  wholeWord: boolean;
}

/** Whitespace as Rust's `char::is_whitespace` sees it (Unicode White_Space). */
const WHITESPACE = /^[\t\n\v\f\r \u{85}\u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}]$/u;

/** Letters and digits, as Rust's `char::is_alphanumeric` sees them. */
const WORD_CHAR = /^[\p{Alphabetic}\p{N}]$/u;

/** One character lowercased on its own, or unchanged if that isn't one character. */
function lowerChar(ch: string): string {
  const lower = ch.toLowerCase();
  // Same length in UTF-16 too, so offsets in the text stay put.
  return lower.length === ch.length && [...lower].length === 1 ? lower : ch;
}

/** The search text for a query: whitespace collapsed and trimmed, lowercased unless matching case. */
export function normalizeQuery(query: string, options: SearchOptions): string {
  return textOf([query.trim()], !options.matchCase).text;
}

/**
 * Text pieces (text nodes, with " " for each `<br>`) joined, whitespace
 * collapsed, and optionally lowercased, with where each UTF-16 unit of the
 * result came from: piece `piece[i]`, unit `offset[i]` within it.
 */
export interface TextMap {
  text: string;
  piece: number[];
  offset: number[];
}

export function textOf(pieces: readonly string[], lower: boolean): TextMap {
  let text = "";
  const piece: number[] = [];
  const offset: number[] = [];
  let inSpace = false;
  pieces.forEach((data, p) => {
    let i = 0;
    for (const ch of data) {
      if (WHITESPACE.test(ch)) {
        if (!inSpace) {
          text += " ";
          piece.push(p);
          offset.push(i);
        }
        inSpace = true;
      } else {
        const out = lower ? lowerChar(ch) : ch;
        text += out;
        for (let u = 0; u < out.length; u++) {
          piece.push(p);
          offset.push(i + u);
        }
        inSpace = false;
      }
      i += ch.length;
    }
  });
  return { text, piece, offset };
}

/** The code point ending just before UTF-16 index `at`, or "" at the start. */
function charBefore(text: string, at: number): string {
  if (at <= 0) return "";
  const last = text.charCodeAt(at - 1);
  const isLowSurrogate = last >= 0xdc00 && last <= 0xdfff;
  return isLowSurrogate && at >= 2 ? text.slice(at - 2, at) : text.slice(at - 1, at);
}

/** The code point starting at UTF-16 index `at`, or "" at the end. */
function charAt(text: string, at: number): string {
  const code = text.codePointAt(at);
  return code === undefined ? "" : String.fromCodePoint(code);
}

/** Start and end (UTF-16 indices) of each match of `needle` in `text`. */
export function findMatches(text: string, needle: string, wholeWord: boolean): Array<[number, number]> {
  const matches: Array<[number, number]> = [];
  if (needle === "") return matches;
  let from = 0;
  for (;;) {
    const start = text.indexOf(needle, from);
    if (start < 0) return matches;
    const end = start + needle.length;
    if (wholeWord && (WORD_CHAR.test(charBefore(text, start)) || WORD_CHAR.test(charAt(text, end)))) {
      // Go on from the next character, as core does.
      from = start + Math.max(1, charAt(text, start).length);
      continue;
    }
    matches.push([start, end]);
    from = end;
  }
}

/** Where a match is: piece and unit offset of its start, and of just after its end. */
export interface MatchSpan {
  startPiece: number;
  startOffset: number;
  endPiece: number;
  endOffset: number;
}

/** Each match of `needle` in the pieces, mapped back to the pieces. */
export function matchSpans(pieces: readonly string[], needle: string, options: SearchOptions): MatchSpan[] {
  const map = textOf(pieces, !options.matchCase);
  return findMatches(map.text, needle, options.wholeWord).map(([start, end]) => ({
    startPiece: map.piece[start] ?? 0,
    startOffset: map.offset[start] ?? 0,
    // A match never ends in a collapsed space's extra characters: a needle
    // can only end in a space if the query did, and queries are trimmed.
    endPiece: map.piece[end - 1] ?? 0,
    endOffset: (map.offset[end - 1] ?? 0) + 1,
  }));
}
