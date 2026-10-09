// Idle-time scheduling with a fallback. WebView2 has requestIdleCallback;
// WebKit-based webviews (macOS, Linux) may not, see DESIGN.md §11.1.

export type CancelIdle = () => void;

/** Runs `task` when the browser is idle (or soon, without idle support). */
export function whenIdle(task: () => void, timeoutMs = 200): CancelIdle {
  if (typeof window.requestIdleCallback === "function") {
    const handle = window.requestIdleCallback(task, { timeout: timeoutMs });
    return () => window.cancelIdleCallback(handle);
  }
  const handle = window.setTimeout(task, 16);
  return () => window.clearTimeout(handle);
}

/** Resolves after the next frame has been painted. */
export function afterPaint(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
  });
}
