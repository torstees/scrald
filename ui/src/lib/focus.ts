// Keyboard focus for dialogs (DESIGN.md §10, accessibility): a modal dialog
// keeps Tab inside itself, and closing any dialog or popover gives focus back
// to whatever had it before, usually the button that opened it.

/** Elements Tab can reach, in document order. */
const FOCUSABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled]):not([type=hidden])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[contenteditable=true]",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

/**
 * Where Tab (or Shift+Tab) goes among `count` focusable elements when focus
 * is at `current` (-1 when it's on none of them): one on, wrapping at
 * either end.
 */
export function nextFocusIndex(count: number, current: number, backwards: boolean): number {
  if (count === 0) return -1;
  if (current < 0) return backwards ? count - 1 : 0;
  return (current + (backwards ? count - 1 : 1)) % count;
}

/**
 * Svelte action: gives focus back to the element that had it when `node`
 * appeared, once `node` goes away (if focus hasn't moved elsewhere since).
 */
export function restoreFocus(node: HTMLElement): { destroy: () => void } {
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  return {
    destroy() {
      const active = document.activeElement;
      const lost = active === null || active === document.body || node.contains(active);
      if (lost && previous?.isConnected) previous.focus({ preventScroll: true });
    },
  };
}

/**
 * Svelte action for a modal dialog: Tab and Shift+Tab cycle through its
 * controls instead of reaching the page behind it, and closing it restores
 * focus as `restoreFocus` does.
 */
export function trapFocus(node: HTMLElement): { destroy: () => void } {
  const restore = restoreFocus(node);

  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== "Tab" || event.ctrlKey || event.altKey || event.metaKey) return;
    const items = [...node.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => el.getClientRects().length > 0);
    const current = items.indexOf(document.activeElement as HTMLElement);
    const next = nextFocusIndex(items.length, current, event.shiftKey);
    event.preventDefault();
    items[next]?.focus();
  }

  node.addEventListener("keydown", onKeydown);
  return {
    destroy() {
      node.removeEventListener("keydown", onKeydown);
      restore.destroy();
    },
  };
}
