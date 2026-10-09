<script lang="ts">
  // The reading view (DESIGN.md §4). Blocks are grouped into heading
  // sections; each mounted section uses `content-visibility: auto` so the
  // browser skips layout and paint for off-screen sections. Sections are
  // mounted outward from the initial position, the rest in idle time.
  import { tick, untrack } from "svelte";
  import type { DocumentModel } from "../lib/types";
  import { assetUrl } from "../lib/commands";
  import { contentId } from "../lib/ids";
  import { afterPaint, whenIdle, type CancelIdle } from "../lib/idle";
  import { batchByBlocks, estimateSectionHeights, mountOrder, sectionBlocks } from "./layout";
  import { blockIndexAtOffset, firstBoxBelow, fractionInto, TOP_ANCHOR, type ScrollAnchor } from "./anchor";

  interface Props {
    doc: DocumentModel;
    /** Token for this document's images in the asset protocol. */
    assetToken: number;
    initialAnchor?: ScrollAnchor;
    /** Called when the section at the top of the viewport changes. */
    onsectionchange?: (sectionId: number) => void;
    /** Called once the first screen has been painted. */
    onfirstscreen?: () => void;
    /** Called once every section is mounted. */
    onfullymounted?: () => void;
    /** Called for clicks on links that aren't in-document anchors. */
    onlink?: (href: string) => void;
    /** Called once scrolling has stopped for a moment (to save the position). */
    onscrollsettled?: () => void;
    /** Reading font size in px (text sizing mode and zoom applied). */
    fontSize?: number | null;
    /** Ignore the measure and let text run the full width. */
    fillWindow?: boolean;
  }

  let {
    doc,
    assetToken,
    initialAnchor = TOP_ANCHOR,
    onsectionchange,
    onfirstscreen,
    onfullymounted,
    onlink,
    onscrollsettled,
    fontSize = null,
    fillWindow = false,
  }: Props = $props();

  /** How long scrolling must pause before the position counts as settled. */
  const SETTLE_MS = 700;
  let settleTimer = 0;

  /** Blocks per idle-time mount batch: enough to finish quickly, small enough not to jank. */
  const BLOCKS_PER_BATCH = 250;

  let scroller: HTMLElement | undefined = $state();
  let mounted = $state<boolean[]>([]);
  const heights = $derived(estimateSectionHeights(doc));

  let cancelIdle: CancelIdle | null = null;
  // Bumped for every new document, so work for an old one stops.
  let generation = 0;
  let currentSection = -1;
  let scrollFrame = 0;

  // (Re)start mounting whenever the document changes.
  $effect(() => {
    const d = doc;
    const anchor = initialAnchor;
    // `untrack` keeps the mount work from subscribing this effect to
    // `mounted`, which it writes.
    untrack(() => void startMount(d, anchor));
    return () => cancelIdle?.();
  });

  async function startMount(d: DocumentModel, anchor: ScrollAnchor): Promise<void> {
    const gen = ++generation;
    cancelIdle?.();
    currentSection = -1;

    const startBlock = d.blocks[blockIndexAtOffset(d.blocks, anchor.offset)];
    const startSection = startBlock?.section ?? 0;
    const batches = batchByBlocks(mountOrder(d.sections.length, startSection), d.sections, BLOCKS_PER_BATCH);

    const flags = d.sections.map(() => false);
    for (const i of batches.shift() ?? []) flags[i] = true;
    mounted = flags;

    await tick();
    if (gen !== generation) return;
    await scrollToAnchor(anchor);
    await afterPaint();
    if (gen !== generation) return;
    onfirstscreen?.();
    noteScrolled();

    const step = () => {
      if (gen !== generation) return;
      const batch = batches.shift();
      if (batch === undefined) {
        cancelIdle = null;
        onfullymounted?.();
        return;
      }
      // Sections above the viewport grow as they mount; the browser's
      // scroll anchoring (overflow-anchor) keeps the visible text still.
      for (const i of batch) mounted[i] = true;
      cancelIdle = whenIdle(step);
    };
    cancelIdle = whenIdle(step);
  }

  function sectionElements(): HTMLElement[] {
    return scroller ? Array.from(scroller.querySelectorAll<HTMLElement>(":scope .sk-section")) : [];
  }

  /** Viewport-relative extent of an element. */
  function extentOf(el: Element | undefined): { top: number; height: number } {
    if (!el) return { top: 0, height: 0 };
    const r = el.getBoundingClientRect();
    return { top: r.top, height: r.height };
  }

  /**
   * The reading position at `offsetY` px below the top of the viewport (the
   * top by default; zoom anchors at the cursor).
   */
  export function captureAnchor(offsetY = 0): ScrollAnchor {
    if (!scroller) return TOP_ANCHOR;
    const viewTop = scroller.getBoundingClientRect().top + offsetY;
    const found = blockAt(viewTop);
    if (!found) return TOP_ANCHOR;
    const block = doc.blocks[found.blockId];
    if (!found.element) return { offset: block?.source.start ?? 0, fraction: 0 };
    const { top, height } = extentOf(found.element);
    return { offset: block?.source.start ?? 0, fraction: fractionInto(viewTop, top, height) };
  }

  /**
   * A block whose last few pixels are all that remain on screen doesn't
   * count as "at" the position. Scroll positions snap to device pixels, so
   * without this a 0.4 px sliver of the previous block could become the
   * anchor, and the paragraph gap after it would then grow with every zoom.
   */
  const SLIVER_PX = 2;

  /**
   * The block at client y-coordinate `y`: the first block whose bottom is
   * more than a sliver below `y`, so the gap between blocks belongs to the
   * block after it. `element` is null when its section isn't mounted yet.
   */
  function blockAt(y: number): { blockId: number; sectionIndex: number; element: HTMLElement | null } | null {
    const sections = sectionElements();
    let s = firstBoxBelow(sections.length, (i) => extentOf(sections[i]), y + SLIVER_PX);
    // Look at most two sections: if `y` is in the trailing
    // gap of section s (below its last block), the answer is in section s + 1.
    for (let attempt = 0; attempt < 2; attempt++) {
      const section = doc.sections[s];
      if (!section) return null;
      const blockEls = sections[s]?.querySelectorAll<HTMLElement>(":scope > .sk-block");
      if (!blockEls || blockEls.length === 0) {
        return { blockId: section.firstBlock, sectionIndex: s, element: null };
      }
      const b = firstBoxBelow(blockEls.length, (i) => extentOf(blockEls[i]), y + SLIVER_PX);
      const element = blockEls[b] ?? null;
      const below = element !== null && element.getBoundingClientRect().bottom > y + SLIVER_PX;
      if (below || s + 1 >= doc.sections.length) {
        return { blockId: section.firstBlock + b, sectionIndex: s, element };
      }
      s += 1;
    }
    return null;
  }

  /** Scrolls so the anchored position is `offsetY` px below the top of the viewport. */
  export async function scrollToAnchor(anchor: ScrollAnchor, offsetY = 0): Promise<void> {
    if (!scroller) return;
    const block = doc.blocks[blockIndexAtOffset(doc.blocks, anchor.offset)];
    if (!block) return;
    if (!mounted[block.section]) {
      mounted[block.section] = true;
      await tick();
    }
    const el = scroller.querySelector<HTMLElement>(`[data-block="${block.id}"]`);
    if (!el) return;
    const { top, height } = extentOf(el);
    scroller.scrollTop += top - scroller.getBoundingClientRect().top - offsetY + anchor.fraction * height;
    noteScrolled();
  }

  /** Width available to the text column: the scroller minus its padding and scrollbar. */
  export function textAreaWidth(): number {
    if (!scroller) return 0;
    const style = getComputedStyle(scroller);
    return scroller.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
  }

  /** Top of the reading viewport, in client coordinates (to anchor zoom at the cursor). */
  export function viewportTop(): number {
    return scroller?.getBoundingClientRect().top ?? 0;
  }

  /** Scrolls a block to the top of the viewport. */
  export function scrollToBlock(blockId: number): Promise<void> {
    const block = doc.blocks[blockId];
    return block ? scrollToAnchor({ offset: block.source.start, fraction: 0 }) : Promise.resolve();
  }

  function updateCurrentSection(): void {
    if (!scroller) return;
    // The section of the block at the top: the same rule as the reading
    // position, so the TOC highlight and the saved position always agree.
    const found = blockAt(scroller.getBoundingClientRect().top);
    if (!found) return;
    const s = found.sectionIndex;
    if (s !== currentSection && doc.sections[s]) {
      currentSection = s;
      onsectionchange?.(s);
    }
  }

  // The last settled reading position. Resizing rewraps text before any
  // resize handler runs, so a position captured then has already moved;
  // restoring this one keeps the reader's place (DESIGN.md §7.4).
  let lastAnchor: ScrollAnchor | null = null;

  /** Refreshes the current section and the recorded reading position. */
  function noteScrolled(): void {
    updateCurrentSection();
    lastAnchor = captureAnchor();
  }

  /** The reading position as of the last scroll or jump (not re-measured now). */
  export function stableAnchor(): ScrollAnchor | null {
    return lastAnchor;
  }

  function onScroll(): void {
    window.clearTimeout(settleTimer);
    settleTimer = window.setTimeout(() => onscrollsettled?.(), SETTLE_MS);
    if (scrollFrame) return;
    scrollFrame = requestAnimationFrame(() => {
      scrollFrame = 0;
      noteScrolled();
    });
  }

  /**
   * Svelte action: points a block's local images at the asset protocol.
   * Core emits `<img data-asset="id">` without a `src` (DESIGN.md §6.2).
   */
  function assetImages(node: HTMLElement, token: number) {
    const apply = (t: number) => {
      for (const img of node.querySelectorAll<HTMLImageElement>("img[data-asset]")) {
        img.src = assetUrl(t, Number(img.dataset.asset));
      }
    };
    apply(token);
    return { update: apply };
  }

  /** Scrolls to the element with HTML id `id` (a heading slug or footnote). */
  export async function scrollToId(id: string): Promise<void> {
    const blockId = blockWithId(id);
    if (blockId === null) return;
    await scrollToBlock(blockId);
    scroller?.querySelector(`[id="${CSS.escape(contentId(id))}"]`)?.scrollIntoView({ block: "start" });
  }

  /** Finds the block whose rendered HTML defines `id` (headings, footnotes). */
  function blockWithId(id: string): number | null {
    const entry = doc.toc.find((t) => t.slug === id);
    if (entry) return entry.blockId;
    const needle = `id="${contentId(id)}"`;
    const block = doc.blocks.find((b) => b.html.includes(needle));
    return block ? block.id : null;
  }

  async function onClick(event: MouseEvent): Promise<void> {
    const link = (event.target as Element | null)?.closest("a");
    const href = link?.getAttribute("href");
    if (!link || href === null || href === undefined) return;
    // Never let a click navigate the webview away from the app.
    event.preventDefault();
    if (href.startsWith("#")) {
      await scrollToId(decodeURIComponent(href.slice(1)));
      return;
    }
    onlink?.(href);
  }
</script>

<!-- The click handler only intercepts clicks on links (delegation), and links are
     keyboard-activatable themselves, so the container needs no key handler. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<div class="sk-scroller" bind:this={scroller} onscroll={onScroll} onclick={onClick} role="document">
  <article class="sk-column" class:fill={fillWindow} style:font-size={fontSize === null ? null : `${fontSize}px`}>
    {#each doc.sections as section, i (section.id)}
      {#if mounted[i]}
        <section
          class="sk-section"
          data-section={section.id}
          style:contain-intrinsic-size={`auto ${heights[i] ?? 0}px`}
        >
          {#each sectionBlocks(doc, section) as block (block.id)}
            <!-- Block.html is sanitized by core (ammonia); it's the only HTML we insert. -->
            <div class="sk-block" data-block={block.id} use:assetImages={assetToken}>{@html block.html}</div>
          {/each}
        </section>
      {:else}
        <section class="sk-section sk-placeholder" data-section={section.id} style:height={`${heights[i] ?? 0}px`}></section>
      {/if}
    {/each}
  </article>
</div>

<style>
  .sk-scroller {
    height: 100%;
    overflow-y: auto;
    overflow-anchor: auto;
    padding: 0 1.5rem;
    box-sizing: border-box;
  }

  /* The text column: at most --sk-measure characters wide (DESIGN.md §7.4). */
  /* "Fill window": text runs the full width, ignoring the measure. */
  .sk-column.fill {
    max-width: 100%;
  }

  .sk-column {
    max-width: min(calc(var(--sk-measure) * 1ch), 100%);
    margin: 0 auto;
    padding: 2.5rem 0 40vh;
  }

  .sk-section {
    content-visibility: auto;
  }

  .sk-placeholder {
    content-visibility: visible;
  }
</style>
