// Contrast results from core (DESIGN.md §7.2), worded for the theme switcher
// and editor.
import type { ContrastProblem } from "../lib/types";

/** The editor's names for the theme's colors. */
export const COLOR_LABELS: Record<string, string> = {
  background: "Page",
  foreground: "Text",
  muted: "Muted text",
  accent: "Accent",
  link: "Links",
  selection: "Selection",
  border: "Borders",
  code_bg: "Code background",
  quote_bar: "Quote bar",
  highlight_bg: "Highlight",
  ui_background: "Sidebar and status bar",
};

/** "Muted text", or "Palette green" for a `[palette]` color. */
export function colorLabel(key: string): string {
  const palette = /^palette\.(\w+)$/.exec(key);
  if (palette) return `Palette ${palette[1]!.replace(/_/g, " ")}`;
  return COLOR_LABELS[key] ?? key;
}

/** "4.15:1": ratios are cut, not rounded, so 4.496 never reads as passing. */
export function formatRatio(ratio: number): string {
  return `${(Math.floor(ratio * 100) / 100).toFixed(2)}:1`;
}

/** The problems that involve the theme color `key`. */
export function problemsFor(key: string, problems: ContrastProblem[]): ContrastProblem[] {
  return problems.filter((p) => p.colors.includes(key));
}

/** One line per problem, for a tooltip. */
export function describeProblems(problems: ContrastProblem[]): string {
  return problems.map((p) => `${p.what}: ${formatRatio(p.ratio)}, needs ${p.minimum}:1`).join("\n");
}
