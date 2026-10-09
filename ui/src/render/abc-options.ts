// Parsing ```abc blocks (issue #133). Compatible with Obsidian's ABC Music
// Notation plugin: optional JSON options at the top, then a `---` line, then
// the tune. Only an allowlist of abcjs options is passed through.

/** Instruments abcjs can draw tablature for. */
const TAB_INSTRUMENTS = new Set(["violin", "fiveString", "mandolin", "guitar"]);

/** abcjs render options a document may set, and the type each must have. */
const ALLOWED: Record<string, "number" | "boolean" | "object" | "tablature"> = {
  tablature: "tablature",
  scale: "number",
  staffwidth: "number",
  wrap: "object",
  paddingtop: "number",
  paddingbottom: "number",
  paddingleft: "number",
  paddingright: "number",
  visualTranspose: "number",
  jazzchords: "boolean",
  germanAlphabet: "boolean",
  oneSvgPerLine: "boolean",
  format: "object",
};

export interface AbcBlock {
  /** The ABC notation to render. */
  tune: string;
  /** Allowed abcjs options from the JSON header (empty if none). */
  options: Record<string, unknown>;
  /** Why the header couldn't be used, if it couldn't (the tune still renders). */
  error: string | null;
}

/** Splits an ```abc block into its optional JSON header and the tune. */
export function parseAbcBlock(text: string): AbcBlock {
  const trimmed = text.replace(/^﻿/, "");
  if (!trimmed.trimStart().startsWith("{")) {
    return { tune: trimmed, options: {}, error: null };
  }
  const lines = trimmed.split(/\r?\n/);
  const separator = lines.findIndex((line) => line.trim() === "---");
  if (separator < 0) {
    return { tune: trimmed, options: {}, error: "Options header has no `---` line after it" };
  }
  const header = lines.slice(0, separator).join("\n");
  const tune = lines.slice(separator + 1).join("\n");
  let parsed: unknown;
  try {
    parsed = JSON.parse(header);
  } catch (e) {
    return { tune, options: {}, error: `Options are not valid JSON: ${(e as Error).message}` };
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    return { tune, options: {}, error: "Options must be a JSON object, like {\"scale\": 1.2}" };
  }
  const { options, ignored } = filterOptions(parsed as Record<string, unknown>);
  const error = ignored.length > 0 ? `Ignored unsupported options: ${ignored.join(", ")}` : null;
  return { tune, options, error };
}

/** Keeps allowlisted options with the right types; reports the rest. */
export function filterOptions(input: Record<string, unknown>): { options: Record<string, unknown>; ignored: string[] } {
  const options: Record<string, unknown> = {};
  const ignored: string[] = [];
  for (const [key, value] of Object.entries(input)) {
    const kind = ALLOWED[key];
    const ok =
      kind === "number"
        ? typeof value === "number" && Number.isFinite(value)
        : kind === "boolean"
          ? typeof value === "boolean"
          : kind === "object"
            ? value !== null && typeof value === "object" && !Array.isArray(value)
            : kind === "tablature"
              ? isTablature(value)
              : false;
    if (ok) options[key] = value;
    else ignored.push(key);
  }
  return { options, ignored };
}

function isTablature(value: unknown): boolean {
  return (
    Array.isArray(value) &&
    value.every(
      (entry) =>
        entry !== null &&
        typeof entry === "object" &&
        TAB_INSTRUMENTS.has(String((entry as Record<string, unknown>).instrument)),
    )
  );
}
