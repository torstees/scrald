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
  easeZoom,
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
  private animation = 0;
  private host: TypographyHost;

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
    const layout = this.layout;
    if (!layout) return;
    this.fontSize = readerFontSize(
      this.mode,
      layout.fontSize,
      {
        availableWidth: this.host.textAreaWidth(),
        measure: layout.measure,
        chPerPx: this.chPerPx,
        minFontSize: layout.minFontSize,
        maxFontSize: layout.maxFontSize,
      },
      this.zoom,
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

  zoomIn(): void {
    void this.animateZoom(stepZoom(this.zoom, 1));
  }

  zoomOut(): void {
    void this.animateZoom(stepZoom(this.zoom, -1));
  }

  resetZoom(): void {
    void this.animateZoom(1);
  }

  /** Ctrl+wheel or trackpad pinch: immediate (the gesture is already smooth), anchored at the cursor. */
  wheel(event: WheelEvent): void {
    cancelAnimationFrame(this.animation);
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

  /** Animates to `target` zoom over ~120 ms, keeping the top of the viewport still. */
  private async animateZoom(target: number): Promise<void> {
    cancelAnimationFrame(this.animation);
    const to = clampZoom(target);
    const from = this.zoom;
    const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const anchor = this.host.captureAnchor(0);
    this.saveDocumentSoon(to);
    if (reduceMotion || from === to) {
      this.docZoom = to;
      this.recompute();
      await this.restore(anchor, 0);
      return;
    }
    const started = performance.now();
    const frame = async (now: number): Promise<void> => {
      const progress = (now - started) / ZOOM_ANIMATION_MS;
      this.docZoom = progress >= 1 ? to : easeZoom(from, to, progress);
      this.recompute();
      await this.restore(anchor, 0);
      if (progress < 1) this.animation = requestAnimationFrame((t) => void frame(t));
    };
    this.animation = requestAnimationFrame((t) => void frame(t));
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
