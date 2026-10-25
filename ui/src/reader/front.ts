// What the reader shows before a document's first block (#189): its cover,
// or else its title when the text doesn't start with it. Core says what the
// document has; the theme says whether to show each, and the user's
// settings can override the theme. Display only: the file never changes.
import type { DocumentModel, FrontChoice, FrontSettings, ImageAsset } from "../lib/types";

/** What to show above the first block: a cover, a title, or nothing. */
export type FrontContent = { kind: "cover"; image: ImageAsset } | { kind: "title"; text: string } | null;

/** The theme's choice, unless the user chose to show or hide it. */
export function frontShown(choice: FrontChoice, themeShows: boolean): boolean {
  return choice === "theme" ? themeShows : choice === "show";
}

/**
 * The cover if the document has one and it's shown; otherwise the missing
 * title if it's shown. A cover that's hidden lets the title take its place.
 */
export function frontContent(
  doc: Pick<DocumentModel, "cover" | "missingTitle">,
  theme: { frontCover: boolean; frontTitle: boolean },
  settings: FrontSettings,
): FrontContent {
  if (doc.cover && frontShown(settings.cover, theme.frontCover)) return { kind: "cover", image: doc.cover };
  if (doc.missingTitle && frontShown(settings.title, theme.frontTitle))
    return { kind: "title", text: doc.missingTitle };
  return null;
}
