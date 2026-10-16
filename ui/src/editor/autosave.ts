/** How long after the last edit autosave waits before saving (DESIGN.md §9.3). */
export const AUTOSAVE_DELAY_MS = 5000;

/**
 * Runs `save` once editing has paused for `delayMs`: every `touch()` (an
 * edit) restarts the wait, so typing never triggers a save mid-sentence.
 */
export class Autosaver {
  private timer: ReturnType<typeof setTimeout> | null = null;
  private readonly delayMs: number;
  private readonly save: () => Promise<void>;

  constructor(delayMs: number, save: () => Promise<void>) {
    this.delayMs = delayMs;
    this.save = save;
  }

  /** An edit happened: save after the pause, unless another edit comes first. */
  touch(): void {
    this.cancel();
    this.timer = setTimeout(() => {
      this.timer = null;
      void this.save();
    }, this.delayMs);
  }

  /** Forget a pending save (autosave turned off, another document opened). */
  cancel(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  }

  get pending(): boolean {
    return this.timer !== null;
  }
}
