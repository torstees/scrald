/** Average adult silent reading speed for prose, in words per minute. */
export const WORDS_PER_MINUTE = 238;

/** "1,234 words" with a locale-independent thousands separator. */
export function formatWordCount(words: number): string {
  const digits = String(Math.max(0, Math.round(words)));
  const grouped = digits.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return `${grouped} ${words === 1 ? "word" : "words"}`;
}

/** Estimated reading time: "< 1 min", "12 min", "3 h 05 min". */
export function formatReadingTime(words: number): string {
  const minutes = Math.round(words / WORDS_PER_MINUTE);
  if (minutes < 1) return "< 1 min";
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  const rest = String(minutes % 60).padStart(2, "0");
  return `${hours} h ${rest} min`;
}
