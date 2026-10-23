// The modifier key for shortcuts: Ctrl on Windows and Linux, Cmd on macOS
// (DESIGN.md §11.1). Shortcuts are written in their Windows form ("Ctrl+S"),
// and these functions turn them into what a Mac user presses and reads.

/** Whether Scrald is running on macOS (WKWebView's user agent says "Macintosh"). */
export const IS_MAC = typeof navigator !== "undefined" && /Mac/.test(navigator.userAgent);

/** The modifier's name as shown to the user: "Ctrl", or "Cmd" on a Mac. */
export const MOD = IS_MAC ? "Cmd" : "Ctrl";

/**
 * Whether a key press or wheel event holds the shortcut modifier: Ctrl, or
 * Cmd on a Mac. On a Mac, Ctrl is left alone: text fields there use it for
 * their own keys (`Ctrl+E` moves to the end of the line).
 */
export function hasMod(event: Pick<KeyboardEvent, "ctrlKey" | "metaKey">, mac = IS_MAC): boolean {
  return mac ? event.metaKey : event.ctrlKey;
}

/**
 * A shortcut as the user should read it: "Ctrl+S" stays as it is, and on a
 * Mac becomes "Cmd+S", with Alt as "Option".
 */
export function keyLabel(keys: string, mac = IS_MAC): string {
  if (!mac) return keys;
  return keys.replace(/\bCtrl\+/g, "Cmd+").replace(/\bAlt\+/g, "Option+");
}
