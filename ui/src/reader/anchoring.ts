// Scroll anchoring for webviews without it (DESIGN.md §11.1). The reader
// relies on the browser keeping the text on screen still when something
// above it changes height: sections mounting in idle time, sections sized by
// `content-visibility` as they come into view, and edits. WebView2 does that
// itself (`overflow-anchor`); where it can't, this does the same by hand.
//
// The block at the top of the screen is the anchor. Whenever the content
// changes size, the anchor's move is undone by scrolling the same distance.
// ResizeObserver callbacks run after layout and before paint, so the move is
// never seen.

/** Where an element is on screen; only `top` is used. */
export interface Placed {
  getBoundingClientRect(): { top: number };
  readonly isConnected: boolean;
}

export interface Scrolled {
  scrollTop: number;
}

export class ScrollKeeper {
  // The anchor and its top in the content's coordinates (on-screen top plus
  // the scroll position), so a scroll in the meantime isn't mistaken for a move.
  private anchor: { element: Placed; at: number } | null = null;
  // The content's height when the anchor was taken.
  private seenSize = 0;

  /**
   * `pick` gives the element at the top of the screen, or null when there's
   * nothing to anchor to; `size` is the content's height.
   */
  constructor(
    private readonly scroller: Scrolled,
    private readonly pick: () => Placed | null,
    private readonly size: () => number,
  ) {}

  /** Takes the anchor at the top of the screen; after every scroll. */
  record(): void {
    // The content changed size since the last look: `correct` is due, and
    // takes the anchor after undoing the move.
    if (this.anchor && this.size() !== this.seenSize) return;
    this.reset();
  }

  /** Scrolls the anchor back to where it was; after the content changes size. */
  correct(): void {
    const anchor = this.anchor;
    // At the very top, stay there: what grew is the start of the document.
    if (anchor && anchor.element.isConnected && this.scroller.scrollTop > 0) {
      const moved = this.contentTop(anchor.element) - anchor.at;
      // Scroll positions snap to device pixels; ignore what that can't fix.
      if (Math.abs(moved) >= 0.5) this.scroller.scrollTop += moved;
    }
    this.reset();
  }

  /**
   * Takes the anchor at the top of the screen as it is now; after the reader
   * puts a position on screen itself (a jump, or keeping its place on zoom).
   */
  reset(): void {
    this.seenSize = this.size();
    const element = this.pick();
    this.anchor = element ? { element, at: this.contentTop(element) } : null;
  }

  private contentTop(element: Placed): number {
    return element.getBoundingClientRect().top + this.scroller.scrollTop;
  }
}
