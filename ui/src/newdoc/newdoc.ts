// The New document dialog's logic (DESIGN.md §11). The app checks the final
// file name (scrald_core::newdoc::file_name); this only suggests one.
import type { FolderChoice, TemplateChoice } from "../lib/types";

/** The built-in template without front matter, used when there's no title. */
export const PLAIN = "builtin:plain";

/** The name a new document starts with. */
export const UNTITLED = "Untitled.md";

/**
 * A file name suggested by a title: characters no file name may hold are
 * dropped, spaces collapsed, and `.md` added. "Notes: day 1?" gives
 * "Notes day 1.md"; an empty title gives "Untitled.md".
 */
export function fileNameForTitle(title: string): string {
  const name = title
    // The characters Windows refuses in file names.
    .replace(/[<>:"/\\|?*]/g, "")
    // Tabs and line breaks become spaces first; other control characters go.
    .replace(/\s+/g, " ")
    .replace(/\p{Cc}/gu, "")
    .trim()
    .replace(/[. ]+$/, "");
  return name === "" ? UNTITLED : `${name}.md`;
}

/** A date as `2026-10-19`, in the user's own time zone. */
export function localDate(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/**
 * The template a folder starts with: the one last used there if it's a
 * remembered folder and that template still exists, else Plain.
 */
export function templateFor(folder: FolderChoice | undefined, templates: TemplateChoice[]): string {
  const id = folder?.remembered ? folder.template : null;
  return id && templates.some((t) => t.id === id) ? id : PLAIN;
}

/** The last part of a folder path, for a short label ("Notes" for C:\Users\me\Notes). */
export function folderName(path: string): string {
  const parts = path.split(/[\\/]+/).filter((p) => p !== "");
  return parts[parts.length - 1] ?? path;
}

/** An app error as a sentence: "there's already…" reads "There's already…". */
export function asSentence(message: string): string {
  return message.charAt(0).toUpperCase() + message.slice(1);
}
