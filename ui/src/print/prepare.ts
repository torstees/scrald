// Getting the reader's DOM ready to print (DESIGN.md §10). Paper can't be
// scrolled or clicked, so everything the screen shows only on demand is
// shown: images load now instead of when scrolled to, and folded callouts
// and `<details>` open. The reader also mounts every section and draws all
// math, diagrams, and music first (`Reader.prepareForPrint`).

import { linkShowsAddress } from "./print";

/** How long to wait for images before printing anyway. */
const IMAGE_WAIT_MS = 10_000;

/** Loads every image in `root` now, waiting until they're ready (or a time limit). */
export async function loadImages(root: ParentNode, waitMs = IMAGE_WAIT_MS): Promise<void> {
  const images = Array.from(root.querySelectorAll<HTMLImageElement>("img"));
  for (const img of images) img.loading = "eager";
  // `decode()` waits for the image to load; a broken image prints as it shows.
  const ready = Promise.all(images.map((img) => img.decode().catch(() => undefined)));
  let timer = 0;
  const limit = new Promise<void>((resolve) => (timer = window.setTimeout(resolve, waitMs)));
  await Promise.race([ready, limit]);
  window.clearTimeout(timer);
}

/**
 * Opens every folded `<details>` (Obsidian's foldable callouts, raw HTML)
 * and marks links whose text is their own address, so a printed address
 * isn't repeated. Returns a function that undoes both.
 */
export function unfold(root: ParentNode): () => void {
  const opened = Array.from(root.querySelectorAll<HTMLDetailsElement>("details:not([open])"));
  for (const details of opened) details.open = true;
  const same = Array.from(root.querySelectorAll<HTMLAnchorElement>("a[href]")).filter((a) =>
    linkShowsAddress(a.textContent ?? "", a.getAttribute("href") ?? ""),
  );
  for (const a of same) a.dataset.skSame = "";
  return () => {
    for (const details of opened) details.open = false;
    for (const a of same) delete a.dataset.skSame;
  };
}
