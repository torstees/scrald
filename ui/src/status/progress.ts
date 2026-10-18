// The reading progress indicator (DESIGN.md §10): how much of the document
// lies above the top of the screen, counted in words rather than pixels.
// Sections the reader hasn't laid out yet only have estimated heights
// (`content-visibility`), so a scroll fraction would drift as they mount.

import { formatReadingTime, WORDS_PER_MINUTE } from "./reading";

/** Where the reader is, as the reader reports it after each scroll. */
export interface ReadingPosition {
  /** Index of the block at the top of the screen. */
  blockIndex: number;
  /** How far the top of the screen is into that block, 0..1. */
  fraction: number;
  /** Whether the end of the document is on screen. */
  atEnd: boolean;
}

/** Words before each block: `before[i]` for block `i`, and the total at the end. */
export function wordsBefore(blocks: readonly { words: number }[]): number[] {
  const before = [0];
  let sum = 0;
  for (const block of blocks) {
    sum += block.words;
    before.push(sum);
  }
  return before;
}

/** The share of the document read, 0..1: 1 once its end is on screen. */
export function readingProgress(before: readonly number[], position: ReadingPosition): number {
  if (position.atEnd) return 1;
  const total = before[before.length - 1] ?? 0;
  if (total === 0) return 0;
  const start = before[position.blockIndex] ?? total;
  const end = before[position.blockIndex + 1] ?? total;
  const fraction = Math.min(1, Math.max(0, position.fraction));
  return Math.min(1, (start + (end - start) * fraction) / total);
}

/** "42%": rounded down, so 100% means the end is on screen. */
export function formatProgress(progress: number): string {
  return `${Math.floor(Math.min(1, Math.max(0, progress)) * 100)}%`;
}

/**
 * Reading time for the words still below the top of the screen: "7 min
 * left", or the whole document's at the start and the end (a short
 * document is at both at once).
 */
export function formatTimeLeft(totalWords: number, progress: number): string {
  if (progress <= 0 || progress >= 1) return formatReadingTime(totalWords);
  const left = totalWords * (1 - progress);
  return left < WORDS_PER_MINUTE / 2 ? "< 1 min left" : `${formatReadingTime(left)} left`;
}
