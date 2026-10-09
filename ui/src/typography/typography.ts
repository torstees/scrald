// Text sizing and zoom (DESIGN.md §7.4). Pure functions; the DOM
// measurements they need are passed in.

export type TextSizing = "fixed" | "fit";

export const MIN_ZOOM = 0.5;
export const MAX_ZOOM = 3;

/** Keyboard zoom steps, like a browser's. */
export const ZOOM_STEPS = [0.5, 0.67, 0.75, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3];

/** How long a keyboard zoom animates, in ms. */
export const ZOOM_ANIMATION_MS = 180;

export function clampZoom(zoom: number): number {
  if (!Number.isFinite(zoom)) return 1;
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
}

/** The next keyboard zoom step above (`direction` 1) or below (-1) `zoom`. */
export function stepZoom(zoom: number, direction: 1 | -1): number {
  const epsilon = 0.001;
  if (direction > 0) {
    return ZOOM_STEPS.find((s) => s > zoom + epsilon) ?? MAX_ZOOM;
  }
  return [...ZOOM_STEPS].reverse().find((s) => s < zoom - epsilon) ?? MIN_ZOOM;
}

/**
 * Zoom after a Ctrl+wheel event (also how trackpad pinch arrives on
 * Windows). Smooth and proportional: one mouse-wheel notch (deltaY 100) is
 * about 16%, and small pinch deltas give small changes.
 */
export function wheelZoom(zoom: number, deltaY: number, deltaMode = 0): number {
  // deltaMode 1 means lines, not pixels; a line is roughly 40 px.
  const pixels = deltaMode === 1 ? deltaY * 40 : deltaY;
  return clampZoom(zoom * Math.pow(1.0015, -pixels));
}

/** The mode in effect: the document's choice, else the user's default, else the theme's suggestion. */
export function resolveTextSizing(
  document: TextSizing | null,
  userDefault: TextSizing | null,
  theme: TextSizing,
): TextSizing {
  return document ?? userDefault ?? theme;
}

/** The zoom in effect: the document's, else the user's default, else 100%. */
export function resolveZoom(document: number | null, userDefault: number | null): number {
  return clampZoom(document ?? userDefault ?? 1);
}

export interface FitInputs {
  /** Width available to the text column, in px. */
  availableWidth: number;
  /** Characters per line the theme wants. */
  measure: number;
  /** Width of one `ch` (the "0" glyph) per px of font size, for the body font. */
  chPerPx: number;
  minFontSize: number;
  maxFontSize: number;
}

/**
 * The font size (px) at which `measure` characters exactly fill the
 * available width, clamped to the theme's range. Below the minimum, text
 * simply rewraps, as in fixed mode.
 */
export function fitFontSize(inputs: FitInputs): number {
  const { availableWidth, measure, chPerPx, minFontSize, maxFontSize } = inputs;
  if (availableWidth <= 0 || measure <= 0 || chPerPx <= 0) return minFontSize;
  const ideal = availableWidth / (measure * chPerPx);
  return Math.min(maxFontSize, Math.max(minFontSize, ideal));
}

/** The reader's font size: the theme's size (fixed) or the fitted size, times zoom. */
export function readerFontSize(mode: TextSizing, themeFontSize: number, fit: FitInputs, zoom: number): number {
  const base = mode === "fit" ? fitFontSize(fit) : themeFontSize;
  return Math.round(base * clampZoom(zoom) * 100) / 100;
}

export function formatZoom(zoom: number): string {
  return `${Math.round(zoom * 100)}%`;
}
