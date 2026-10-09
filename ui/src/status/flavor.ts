// Labels for the status bar flavor switcher (DESIGN.md §5.2).

import type { Flavor, FlavorSource } from "../lib/types";

export const FLAVOR_NAMES: Record<Flavor, string> = {
  gfm: "GFM",
  obsidian: "Obsidian",
  pandoc: "Pandoc",
};

/** The switcher's value: a flavor the user chose, or "auto" for everything else. */
export function switcherValue(flavor: Flavor, source: FlavorSource): Flavor | "auto" {
  return source === "document" ? flavor : "auto";
}

/** Label for the "auto" option, saying what it currently resolves to and why. */
export function autoLabel(flavor: Flavor, source: FlavorSource): string {
  const name = FLAVOR_NAMES[flavor];
  switch (source) {
    case "frontMatter":
      return `Auto: ${name} (front matter)`;
    case "folder":
      return `Auto: ${name} (.scrald.toml)`;
    case "detected":
      return `Auto: ${name} (detected)`;
    default:
      return `Auto: ${name}`;
  }
}
