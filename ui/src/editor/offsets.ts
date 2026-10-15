// Converting between the byte offsets core uses (UTF-8, DESIGN.md §3) and
// the positions CodeMirror and JavaScript strings use (UTF-16 code units).
// Raw mode carries the reading position across with these (§9.2).

/** UTF-8 length of one code point. */
function utf8Length(codePoint: number): number {
  if (codePoint < 0x80) return 1;
  if (codePoint < 0x800) return 2;
  if (codePoint < 0x10000) return 3;
  return 4;
}

/**
 * The string index at UTF-8 byte offset `byteOffset` in `text`. An offset
 * inside a character rounds back to that character's start; past the end
 * gives `text.length`.
 */
export function byteToIndex(text: string, byteOffset: number): number {
  let bytes = 0;
  let i = 0;
  while (i < text.length) {
    const codePoint = text.codePointAt(i) ?? 0;
    const size = utf8Length(codePoint);
    if (bytes + size > byteOffset) return i;
    bytes += size;
    // Characters outside the BMP take two UTF-16 code units.
    i += codePoint > 0xffff ? 2 : 1;
  }
  return text.length;
}

/** The UTF-8 byte offset of string index `index` in `text`. */
export function indexToByte(text: string, index: number): number {
  let bytes = 0;
  let i = 0;
  while (i < index && i < text.length) {
    const codePoint = text.codePointAt(i) ?? 0;
    bytes += utf8Length(codePoint);
    i += codePoint > 0xffff ? 2 : 1;
  }
  return bytes;
}

/**
 * The CodeMirror position for string index `index` of `text`, when the
 * editor's line separator is `separator`: CodeMirror counts each line break
 * as one position, so with "\r\n" every CRLF before `index` is one less.
 */
export function indexToEditorPos(text: string, index: number, separator: "\n" | "\r\n"): number {
  if (separator === "\n") return index;
  let crlfs = 0;
  let from = 0;
  for (;;) {
    const at = text.indexOf("\r\n", from);
    if (at === -1 || at + 2 > index) break;
    crlfs += 1;
    from = at + 2;
  }
  return index - crlfs;
}

/** UTF-8 length of `text`: the byte offset of its end. */
export function byteLength(text: string): number {
  return new TextEncoder().encode(text).length;
}
