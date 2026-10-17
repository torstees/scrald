// Small pure helpers for the theme editor (ThemeEditor.svelte).

import type { ThemeChange } from "../lib/types";

/**
 * What a color field's text means: `null` (empty: remove the key, so the
 * color is derived), a normalized hex color, or `undefined` while it isn't
 * a color yet (nothing is sent).
 */
export function colorFieldValue(text: string): string | null | undefined {
  const trimmed = text.trim();
  if (trimmed === "") return null;
  const hex = trimmed.startsWith("#") ? trimmed : `#${trimmed}`;
  return /^#([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/i.test(hex) ? hex.toLowerCase() : undefined;
}

/** `#rrggbb` for an `<input type="color">`, which has no alpha. */
export function colorPickerValue(color: string): string {
  const hex = color.replace(/^#/, "");
  if (hex.length === 3 || hex.length === 4) {
    return `#${[...hex.slice(0, 3)].map((c) => c + c).join("")}`.toLowerCase();
  }
  return `#${hex.slice(0, 6)}`.toLowerCase();
}

/** A picker's `#rrggbb` with the alpha of `previous` kept (`#rrggbbaa`). */
export function withAlphaOf(picked: string, previous: string | null): string {
  const hex = previous?.replace(/^#/, "") ?? "";
  const alpha = hex.length === 8 ? hex.slice(6) : hex.length === 4 ? hex[3]! + hex[3]! : "";
  return `${picked}${alpha}`.toLowerCase();
}

/** "400, 700" → [400, 700]; anything that isn't a weight is dropped. */
export function parseWeights(text: string): number[] {
  return text
    .split(/[\s,]+/)
    .map((w) => Number(w))
    .filter((w) => Number.isInteger(w) && w >= 1 && w <= 1000);
}

/** "Charter, Georgia" → ["Charter", "Georgia"]. */
export function parseList(text: string): string[] {
  return text
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s !== "");
}

/**
 * The editor's pending changes, by key, with a new one recorded. Removing a
 * whole font slot drops changes inside it; changing inside a slot cancels a
 * pending removal of it.
 */
export function recordChange(
  changes: ReadonlyMap<string, ThemeChange["value"]>,
  key: string,
  value: ThemeChange["value"],
): Map<string, ThemeChange["value"]> {
  const next = new Map(changes);
  if (value === null) {
    for (const k of [...next.keys()]) if (k.startsWith(`${key}.`)) next.delete(k);
  }
  const parent = key.split(".").slice(0, -1).join(".");
  if (parent.startsWith("fonts.") && next.get(parent) === null) next.delete(parent);
  // Re-insert so the order of changes follows the order of edits.
  next.delete(key);
  next.set(key, value);
  return next;
}

/** Pending changes as the list the app takes. */
export function changeList(changes: ReadonlyMap<string, ThemeChange["value"]>): ThemeChange[] {
  return [...changes].map(([key, value]) => ({ key, value }));
}
