// The status bar's line-ending label and its menu (#219, #220).
import type { LineEnding } from "../lib/types";

/** What the status bar shows: "LF", "CRLF", or "Mixed", with "BOM" after it when the file has one. */
export function lineEndingLabel(lineEnding: LineEnding, mixed: boolean, hasBom: boolean): string {
  const ending = mixed ? "Mixed" : lineEnding === "crlf" ? "CRLF" : "LF";
  return hasBom ? `${ending} · BOM` : ending;
}

/** The line endings the document can be converted to: the other one, or both for a mixed file. */
export function conversions(lineEnding: LineEnding, mixed: boolean): LineEnding[] {
  if (mixed) return ["lf", "crlf"];
  return lineEnding === "crlf" ? ["lf"] : ["crlf"];
}

/** A summary of what tidying changed, for the banner. */
export function tidySummary(joined: number, split: number): string {
  if (joined === 0 && split === 0) return "No hard-wrapped lines to tidy";
  const parts: string[] = [];
  if (joined > 0) parts.push(`joined ${joined} ${joined === 1 ? "line" : "lines"}`);
  if (split > 0) parts.push(`started ${split} new ${split === 1 ? "paragraph" : "paragraphs"}`);
  const text = parts.join(" and ");
  return text.charAt(0).toUpperCase() + text.slice(1);
}
