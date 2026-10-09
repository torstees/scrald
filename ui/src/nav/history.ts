// Back/forward history for documents opened in one window (DESIGN.md §10).
// Each entry remembers its reading position, so Back returns to the exact spot.

import type { ScrollAnchor } from "../reader/anchor";

export interface HistoryEntry {
  path: string;
  anchor: ScrollAnchor;
}

export class NavHistory {
  private entries: HistoryEntry[] = [];
  private index = -1;

  get current(): HistoryEntry | null {
    return this.entries[this.index] ?? null;
  }

  get canGoBack(): boolean {
    return this.index > 0;
  }

  get canGoForward(): boolean {
    return this.index < this.entries.length - 1;
  }

  /** Records a newly opened document; anything ahead of it is dropped. */
  push(entry: HistoryEntry): void {
    this.entries = this.entries.slice(0, this.index + 1);
    this.entries.push(entry);
    this.index = this.entries.length - 1;
  }

  /** Updates the reading position of the current entry (before leaving it). */
  updateAnchor(anchor: ScrollAnchor): void {
    const entry = this.entries[this.index];
    if (entry) entry.anchor = anchor;
  }

  back(): HistoryEntry | null {
    if (!this.canGoBack) return null;
    this.index -= 1;
    return this.current;
  }

  forward(): HistoryEntry | null {
    if (!this.canGoForward) return null;
    this.index += 1;
    return this.current;
  }
}
