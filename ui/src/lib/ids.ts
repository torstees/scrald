/**
 * Prefix core adds to every HTML id from document content (render.rs
 * `ID_PREFIX`), so content can't clash with the app's own element ids.
 * Links in documents use the bare id (`#intro`, `#fn-1`).
 */
export const CONTENT_ID_PREFIX = "user-content-";

/** The DOM id for a bare document id or heading slug. */
export function contentId(id: string): string {
  return id.startsWith(CONTENT_ID_PREFIX) ? id : CONTENT_ID_PREFIX + id;
}
