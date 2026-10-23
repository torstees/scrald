// Printing (DESIGN.md §10): the CSS Scrald adds while a document prints.
// The page's own print rules (app.css, `@media print`) hide the app around
// the document; this adds what changes with each print: the title in the
// page margins, the colors, and link addresses.

/** The two ways to color a printed page. */
export type PrintColors = "theme" | "paper";

export interface PrintOptions {
  colors: PrintColors;
  /** Print each web link's address after its text. */
  linkAddresses: boolean;
}

/** How the title and page numbers in the margins look. */
const MARGIN_TEXT = "font-family: var(--sk-font-ui); font-size: 9pt; color: var(--sk-color-muted);";

/** The theme whose colors "White paper" borrows. */
export const PAPER_THEME = "scrald-light";

/** What a print starts with: white paper for a dark theme, the theme's own colors otherwise. */
export function defaultPrintOptions(appearance: string | undefined): PrintOptions {
  return { colors: appearance === "dark" ? "paper" : "theme", linkAddresses: false };
}

/** `text` as a quoted CSS string, safe inside `content: …`. */
export function cssString(text: string): string {
  let out = "";
  for (const ch of text) {
    if (ch === '"' || ch === "\\") out += `\\${ch}`;
    // Control characters (a line break in a title) become escapes; the space
    // ends the escape, so a following hex digit isn't read as part of it.
    else if (ch < " " || ch === "\u007f") out += `\\${ch.codePointAt(0)?.toString(16)} `;
    else out += ch;
  }
  return `"${out}"`;
}

/**
 * The color declarations of a theme's CSS (`color-scheme`, `--sk-color-*`,
 * `--sk-palette-*`): for "White paper", another theme's colors without its
 * fonts or layout.
 */
export function colorDeclarations(themeCss: string): string[] {
  const root = /:root\s*\{([^}]*)\}/.exec(themeCss)?.[1] ?? "";
  return root
    .split(";")
    .map((d) => d.trim())
    .filter((d) => /^(color-scheme|--sk-color-[\w-]+|--sk-palette-[\w-]+)\s*:/.test(d));
}

/**
 * The CSS for one print: the document's title at the top of every page but
 * the first (which starts with the title anyway) and the page number at the
 * bottom, in the page margins; the colors, page edges included; and, if
 * asked, link addresses.
 * `paperColors` are the declarations from `colorDeclarations`, for "White
 * paper"; they apply on screen too while printing, so diagrams are drawn
 * in them.
 */
export function printCss(title: string, options: PrintOptions, paperColors: string[]): string {
  const lines: string[] = [];
  if (options.colors === "paper" && paperColors.length > 0) {
    // Scrald Light's page is cream; on paper it's the paper's own white.
    lines.push(`:root { ${[...paperColors, "--sk-color-background: #ffffff"].join("; ")}; }`);
  }
  lines.push(
    "@media print {",
    // Backgrounds print without the print dialog's "Background graphics":
    // a theme's page color and code backgrounds are part of how it looks.
    "  html, body { print-color-adjust: exact; -webkit-print-color-adjust: exact; }",
    "  @page {",
    "    margin: 18mm 16mm;",
    // The page color reaches the edges, not just the text area.
    "    background-color: var(--sk-color-background);",
    `    @top-center { content: ${cssString(title)}; ${MARGIN_TEXT} }`,
    `    @bottom-center { content: counter(page) " / " counter(pages); ${MARGIN_TEXT} }`,
    "  }",
    // An empty box rather than none: the browser fills an edge without one
    // with its own header (the date and the page's title).
    '  @page :first { @top-center { content: ""; } }',
  );
  if (options.linkAddresses) {
    // Web and mail links only: an in-document anchor or a local path means
    // nothing on paper. A link whose text is already its address is skipped.
    lines.push(
      '  .sk-block a:is([href^="http:"], [href^="https:"], [href^="mailto:"]):not([data-sk-same])::after {',
      '    content: "(" attr(href) ")";',
      // An inline block isn't underlined by the link it's in.
      "    display: inline-block;",
      "    margin-left: 0.25em;",
      "    text-decoration: none;",
      "    font-size: 0.85em;",
      "    overflow-wrap: anywhere;",
      "    color: var(--sk-color-muted);",
      "  }",
    );
  }
  lines.push("}");
  return lines.join("\n");
}

/** Whether a link's text is its address already (printing it twice adds nothing). */
export function linkShowsAddress(text: string, href: string): boolean {
  const bare = (s: string) =>
    s
      .trim()
      .replace(/^mailto:/i, "")
      .replace(/^https?:\/\//i, "")
      .replace(/\/$/, "");
  return bare(text) === bare(href);
}
