// Helpers for the start screen's document lists (DESIGN.md §11).

import type { RecentDocument } from "../lib/types";

/** Extensions Scrald opens (mirrors `MARKDOWN_EXTENSIONS` in commands.rs). */
export const MARKDOWN_EXTENSIONS = ["md", "markdown", "mdown", "mkd", "mkdn", "txt"];

/** Whether a dropped file looks like something Scrald can open. */
export function isMarkdownPath(path: string): boolean {
  const name = fileName(path);
  const dot = name.lastIndexOf(".");
  return dot > 0 && MARKDOWN_EXTENSIONS.includes(name.slice(dot + 1).toLowerCase());
}

/** The last part of a path, with either separator. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** The stored title, or the file name without its extension. */
export function displayTitle(doc: RecentDocument): string {
  if (doc.title) return doc.title;
  const name = fileName(doc.path);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

/** Pinned documents and the rest, each in the order given. */
export function splitPinned(docs: RecentDocument[]): { pinned: RecentDocument[]; recent: RecentDocument[] } {
  return { pinned: docs.filter((d) => d.pinned), recent: docs.filter((d) => !d.pinned) };
}

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** "just now", "5 minutes ago", "yesterday", "3 days ago", or a date. */
export function lastOpenedText(when: number, now: number): string {
  const ago = now - when;
  if (ago < MINUTE) return "just now";
  if (ago < HOUR) return plural(Math.floor(ago / MINUTE), "minute") + " ago";
  if (ago < DAY) return plural(Math.floor(ago / HOUR), "hour") + " ago";
  const days = calendarDaysBetween(when, now);
  if (days <= 1) return "yesterday";
  if (days < 7) return `${days} days ago`;
  return new Date(when).toLocaleDateString(undefined, { dateStyle: "medium" });
}

function plural(n: number, unit: string): string {
  return `${n} ${unit}${n === 1 ? "" : "s"}`;
}

/** Midnights crossed between two times, in local time. */
function calendarDaysBetween(earlier: number, later: number): number {
  const start = new Date(earlier);
  const end = new Date(later);
  const startDay = Date.UTC(start.getFullYear(), start.getMonth(), start.getDate());
  const endDay = Date.UTC(end.getFullYear(), end.getMonth(), end.getDate());
  return Math.round((endDay - startDay) / DAY);
}
