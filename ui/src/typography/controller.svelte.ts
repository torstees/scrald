// Text sizing and zoom state for one window (DESIGN.md §7.4): resolves the
// mode and zoom, computes the reader's font size, keeps the reading position
// steady while sizes change, and remembers choices.
//
// This is a `.svelte.ts` module so its class fields can use runes ($state,
// $derived) and stay reactive in components that read them.

import { tick } from "svelte";
import { setDocumentTypography, setTypographyDefaults, typographyDefaults } from "../lib/commands";
import type { DocumentMemory, ThemeLayout, TypographyDefaults } from "../lib/types";
import type { ScrollAnchor } from "../reader/anchor";
import {
  clampZoom,
  formatZoom,
  readerFontSize,
  resolveTextSizing,
  resolveZoom,
  stepZoom,
  wheelZoom,
  ZOOM_ANIMATION_MS,
  type TextSizing,
} from "./typography";

/** What the controller needs from the page. */
export interface TypographyHost {
  /** The current document's path, for saving per-document choices. */
  documentPath(): string | null;
  /** Width the text column may use, in px. */
  textAreaWidth(): number;
  /** Top of the reading viewport in client coordinates. */
  viewportTop(): number;
  captureAnchor(offsetY: number): ScrollAnchor | null;
  /** The position as of the last scroll, from before any reflow in progress. */
  stableAnchor(): ScrollAnchor | null;
  restoreAnchor(anchor: ScrollAnchor, offsetY: number): Promise<void>;
  /**
   * Visually scales the reading column by `scale` (a CSS transform, no
   * reflow) over `durationMs`, keeping the point `offsetY` px below the top
   * of the viewport still. Resolves when the transition ends.
   */
  previewScale(scale: number, offsetY: number, durationMs: number): Promise<void>;
  /** Removes the preview transform instantly. */
  clearPreview(): void;
}

/** Wait this long after the last change before saving (wheel zoom fires a lot). */
const SAVE_DELAY_MS = 400;

export class TypographyController {
  /** This document's own choices; null means "not set, use the default". */
  docTextSizing = $state<TextSizing | null>(null);
  docZoom = $state<number | null>(null);
  defaults = $state<TypographyDefaults>({ textSizing: null, zoom: null, fillWindow: false });
  layout = $state<ThemeLayout | null>(null);
  /** The reader's font size in px, or null before a theme is known. */
  fontSize = $state<number | null>(null);

  mode = $derived(resolveTextSizing(this.docTextSizing, this.defaults.textSizing, this.layout?.textSizing ?? "fixed"));
  zoom = $derived(resolveZoom(this.docZoom, this.defaults.zoom));
  fillWindow = $derived(this.defaults.fillWindow);
  summary = $derived(`${this.mode === "fit" ? "Fit" : "Fixed"} · ${formatZoom(this.zoom)}`);

  /** Width of one `ch` per px of font size for the body font (measured). */
  private chPerPx = 0.5;
  private saveTimer = 0;
  private host: TypographyHost;
  /** Keyboard zoom in progress: its target, and the anchor captured when it began. */
  private pendingZoom: number | null = null;
  private pendingAnchor: ScrollAnchor | null = null;
  /** Bumped by every zoom, so a superseded animation knows to stop. */
  private zoomGeneration = 0;

  constructor(host: TypographyHost) {
    this.host = host;
  }

  async loadDefaults(): Promise<void> {
    try {
      this.defaults = await typographyDefaults();
    } catch (e) {
      console.warn("could not load typography defaults", e);
    }
  }

  /** A newly opened document's remembered choices. */
  setDocument(memory: DocumentMemory): void {
    this.docTextSizing = memory.textSizing;
    this.docZoom = memory.zoom;
  }

  /** A theme was applied: its layout numbers and body font change the sizes. */
  setTheme(layout: ThemeLayout): void {
    this.layout = layout;
    this.measureCh();
    this.recompute();
    // Web fonts may finish loading later and change the character width.
    void document.fonts.ready.then(() => {
      const before = this.chPerPx;
      this.measureCh();
      if (Math.abs(before - this.chPerPx) > 0.001 && this.mode === "fit") void this.keepPlace(0, () => this.recompute());
    });
  }

  /** Recomputes the font size from the current mode, zoom, theme, and width. */
  recompute(): void {
    const size = this.sizeAt(this.zoom);
    if (size !== null) this.fontSize = size;
  }

  /** The reading font size at `zoom` with the current mode, theme, and width. */
  private sizeAt(zoom: number): number | null {
    const layout = this.layout;
    if (!layout) return null;
    return readerFontSize(
      this.mode,
      layout.fontSize,
      {
        availableWidth: this.host.textAreaWidth(),
        measure: layout.measure,
        chPerPx: this.chPerPx,
        minFontSize: layout.minFontSize,
        maxFontSize: layout.maxFontSize,
      },
      zoom,
    );
  }

  /**
   * The window or sidebar changed size. Text has already rewrapped by now,
   * so restore the position recorded before the resize (both modes), after
   * refitting the font size in fit mode.
   */
  onResize(): void {
    const anchor = this.host.stableAnchor();
    if (this.mode === "fit") this.recompute();
    void this.restore(anchor, 0);
  }

  setMode(mode: TextSizing): void {
    this.docTextSizing = mode;
    void this.keepPlace(0, () => this.recompute());
    this.saveDocument();
  }

  // Quick repeated presses step from the zoom already on its way, not the
  // one on screen, so three presses always mean three steps.
  zoomIn(): void {
    void this.animateZoom(stepZoom(this.pendingZoom ?? this.zoom, 1));
  }

  zoomOut(): void {
    void this.animateZoom(stepZoom(this.pendingZoom ?? this.zoom, -1));
  }

  resetZoom(): void {
    void this.animateZoom(1);
  }

  /** Ctrl+wheel or trackpad pinch: immediate (the gesture is already smooth), anchored at the cursor. */
  wheel(event: WheelEvent): void {
    this.finishZoomNow();
    const offsetY = Math.max(0, event.clientY - this.host.viewportTop());
    const target = wheelZoom(this.zoom, event.deltaY, event.deltaMode);
    void this.keepPlace(offsetY, () => {
      this.docZoom = target;
      this.recompute();
    });
    this.saveDocument();
  }

  async setFillWindow(fill: boolean): Promise<void> {
    await this.keepPlace(0, () => {
      this.defaults = { ...this.defaults, fillWindow: fill };
    });
    await this.saveDefaults();
  }

  /** Promotes the current mode and zoom to the global default (DESIGN.md §7.4). */
  async makeDefault(): Promise<void> {
    this.defaults = { textSizing: this.mode, zoom: this.zoom, fillWindow: this.fillWindow };
    await this.saveDefaults();
  }

  /**
   * Zooms to `target`, keeping the top of the viewport still. The animation
   * is a CSS transform on the reading column (smooth, no text reflow); when
   * it ends, the real font size is applied and the transform removed in the
   * same frame, so the swap is invisible. Laying text out on every frame
   * instead was too slow to look smooth.
   */
  private async animateZoom(target: number): Promise<void> {
    const generation = ++this.zoomGeneration;
    const to = clampZoom(target);
    // Chained presses keep the anchor from the first one.
    const anchor = this.pendingAnchor ?? this.host.captureAnchor(0);
    this.pendingAnchor = anchor;
    this.pendingZoom = to;
    this.saveDocumentSoon(to);

    const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const from = this.fontSize;
    const goal = this.sizeAt(to);
    if (!reduceMotion && from !== null && goal !== null && from > 0) {
      // The transform is relative to the font size actually laid out, so a
      // second press mid-animation glides on from wherever the first got to.
      await this.host.previewScale(goal / from, 0, ZOOM_ANIMATION_MS);
      if (generation !== this.zoomGeneration) return;
    }
    await this.applyPendingZoom();
  }

  /** Applies the pending keyboard zoom for real and clears the preview. */
  private async applyPendingZoom(): Promise<void> {
    const to = this.pendingZoom;
    const anchor = this.pendingAnchor;
    this.pendingZoom = null;
    this.pendingAnchor = null;
    if (to === null) return;
    this.docZoom = to;
    this.recompute();
    // Let Svelte write the new font size, then drop the transform and fix
    // the scroll position before the browser paints.
    await tick();
    this.host.clearPreview();
    await this.restore(anchor, 0);
  }

  /** Ends any keyboard zoom animation at once (another zoom gesture took over). */
  private finishZoomNow(): void {
    if (this.pendingZoom === null) return;
    this.zoomGeneration++;
    void this.applyPendingZoom();
  }

  /** Runs `change`, then puts the reading position back where it was. */
  private async keepPlace(offsetY: number, change: () => void): Promise<void> {
    const anchor = this.host.captureAnchor(offsetY);
    change();
    await this.restore(anchor, offsetY);
  }

  private async restore(anchor: ScrollAnchor | null, offsetY: number): Promise<void> {
    if (!anchor) return;
    await tick();
    await this.host.restoreAnchor(anchor, offsetY);
  }

  /** Measures the body font's `ch` width at a known size. */
  private measureCh(): void {
    const probe = document.createElement("div");
    probe.style.cssText =
      "position:absolute;visibility:hidden;left:-9999px;top:0;font-family:var(--sk-font-body);font-size:100px;width:100ch";
    document.body.appendChild(probe);
    const width = probe.getBoundingClientRect().width;
    probe.remove();
    if (width > 0) this.chPerPx = width / 10000;
  }

  private saveDocument(): void {
    this.saveDocumentSoon(this.docZoom);
  }

  private saveDocumentSoon(zoom: number | null): void {
    const path = this.host.documentPath();
    if (!path) return;
    window.clearTimeout(this.saveTimer);
    const textSizing = this.docTextSizing;
    this.saveTimer = window.setTimeout(() => setDocumentTypography(path, textSizing, zoom), SAVE_DELAY_MS);
  }

  private async saveDefaults(): Promise<void> {
    try {
      await setTypographyDefaults(this.defaults);
    } catch (e) {
      console.warn("could not save typography defaults", e);
    }
  }
}
