// What the properties panel shows (DESIGN.md §8.2): recognized properties in
// a fixed order, then every other key in file order, each with whether the
// minimal-edit YAML engine can change it (§8.3).
import type { FieldInfo, FrontMatter, PropertyKeys } from "../lib/types";

export type PropertyKind = "text" | "multiline" | "tags" | "list" | "link" | "value";

export interface PropertyRow {
  /** Shown to the reader. */
  label: string;
  /** The YAML key it comes from. */
  key: string;
  kind: PropertyKind;
  /** Text for text-like kinds, items for lists and tags. */
  value: string | string[];
  editable: boolean;
  /** Why it can't be edited in the panel, when it can't. */
  reason: string | null;
}

/** Recognized properties, in display order. */
const RECOGNIZED: { property: keyof PropertyKeys; label: string; kind: PropertyKind }[] = [
  { property: "title", label: "Title", kind: "text" },
  { property: "authors", label: "Author", kind: "list" },
  { property: "summary", label: "Summary", kind: "multiline" },
  { property: "tags", label: "Tags", kind: "tags" },
  { property: "notes", label: "Notes", kind: "multiline" },
  { property: "source", label: "Source", kind: "link" },
  { property: "assets", label: "Assets", kind: "text" },
  { property: "theme", label: "Theme", kind: "text" },
  { property: "flavor", label: "Flavor", kind: "text" },
];

function recognizedValue(fm: FrontMatter, property: keyof PropertyKeys): string | string[] | null {
  switch (property) {
    case "authors":
      return fm.authors.length > 0 ? fm.authors : null;
    case "tags":
      return fm.tags.length > 0 ? fm.tags : null;
    default:
      return fm[property];
  }
}

function fieldFor(fields: FieldInfo[], key: string): FieldInfo | undefined {
  return fields.find((f) => f.key === key);
}

/** The panel's rows: recognized properties that are set, then other keys. */
export function propertyRows(fm: FrontMatter): PropertyRow[] {
  const rows: PropertyRow[] = [];
  for (const { property, label, kind } of RECOGNIZED) {
    const key = fm.keys[property];
    const value = recognizedValue(fm, property);
    if (key === null || value === null) continue;
    const field = fieldFor(fm.fields, key);
    rows.push({
      label: kind === "list" && Array.isArray(value) && value.length > 1 ? "Authors" : label,
      key,
      kind,
      value,
      editable: field?.editable ?? false,
      reason: field?.reason ?? null,
    });
  }
  for (const [key, raw] of Object.entries(fm.extra)) {
    const field = fieldFor(fm.fields, key);
    const list = scalarList(raw);
    rows.push({
      label: key,
      key,
      kind: list === null ? "value" : "list",
      value: list ?? formatValue(raw),
      editable: field?.editable ?? false,
      reason: field?.reason ?? null,
    });
  }
  return rows;
}

/** A list of scalars as text items, or null if `value` isn't one. */
function scalarList(value: unknown): string[] | null {
  if (!Array.isArray(value)) return null;
  if (!value.every((v) => ["string", "number", "boolean"].includes(typeof v))) return null;
  return value.map((v) => String(v));
}

/** Any YAML value as short readable text: maps as `a: 1, b: 2`. */
export function formatValue(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (Array.isArray(value)) return value.map(formatValue).join(", ");
  if (typeof value === "object") {
    return Object.entries(value as Record<string, unknown>)
      .map(([k, v]) => `${k}: ${formatValue(v)}`)
      .join(", ");
  }
  return String(value);
}

/** Whether a link value is safe to show as a link (opened by the app's link handler). */
export function isWebLink(text: string): boolean {
  return /^https?:\/\//i.test(text.trim());
}
