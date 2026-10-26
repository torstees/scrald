// Which of the web features the reader leans on this webview has (DESIGN.md
// §11.1). WebView2 has them all; WebKit (WKWebView on macOS, WebKitGTK on
// Linux) has had some only recently, and each has a fallback where it's used.
// A test build can force the fallbacks on with VITE_SCRALD_FALLBACKS ("all",
// or a comma list of the names below), to try them on Windows.

export type Feature = "idleCallback" | "contentVisibility" | "scrollAnchoring" | "customHighlights";

export const FEATURES: readonly Feature[] = [
  "idleCallback",
  "contentVisibility",
  "scrollAnchoring",
  "customHighlights",
];

/** The features a VITE_SCRALD_FALLBACKS value forces off. */
export function forcedOff(setting: string | undefined): Set<Feature> {
  const names = (setting ?? "")
    .split(",")
    .map((name) => name.trim())
    .filter((name) => name !== "");
  if (names.includes("all")) return new Set(FEATURES);
  return new Set(FEATURES.filter((feature) => names.includes(feature)));
}

function detect(feature: Feature): boolean {
  if (typeof window === "undefined") return false;
  const css = typeof CSS !== "undefined" && typeof CSS.supports === "function";
  switch (feature) {
    case "idleCallback":
      return typeof window.requestIdleCallback === "function";
    case "contentVisibility":
      return css && CSS.supports("content-visibility", "auto");
    case "scrollAnchoring":
      return css && CSS.supports("overflow-anchor", "auto");
    case "customHighlights":
      return typeof CSS !== "undefined" && "highlights" in CSS && typeof Highlight !== "undefined";
  }
}

let cache: Map<Feature, boolean> | null = null;

/** Whether this webview has `feature` (and it isn't forced off). */
export function supports(feature: Feature): boolean {
  if (cache === null) {
    const off = forcedOff(import.meta.env.VITE_SCRALD_FALLBACKS);
    cache = new Map(FEATURES.map((f) => [f, !off.has(f) && detect(f)]));
  }
  return cache.get(feature) ?? false;
}

/** The features this webview lacks, for the log. */
export function missingFeatures(): Feature[] {
  return FEATURES.filter((feature) => !supports(feature));
}
