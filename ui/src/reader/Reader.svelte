<script lang="ts">
  // The reading view (DESIGN.md §4). Blocks are grouped into heading
  // sections; each mounted section uses `content-visibility: auto` so the
  // browser skips layout and paint for off-screen sections. Sections are
  // mounted outward from the initial position, the rest in idle time.
  import { onDestroy, tick, untrack } from "svelte";
  import type { Block, DocumentModel } from "../lib/types";
  import BlockEditor from "../editor/BlockEditor.svelte";
  import type { BlockEditing } from "../editor/blocks";
  import { assetUrl } from "../lib/commands";
  import { contentId } from "../lib/ids";
  import { richContent } from "../render/renderers";
  import PropertiesPanel from "../frontmatter/PropertiesPanel.svelte";
  import type { PropertyEditor } from "../frontmatter/properties";
  import { afterPaint, whenIdle, type CancelIdle } from "../lib/idle";
  import { batchByBlocks, blockKeys, estimateSectionHeights, mountOrder, sectionBlocks } from "./layout";
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
    /** Whether the properties panel is open. */
    propertiesOpen?: boolean;
    /** Makes the properties panel editable. */
    propertyEditor?: PropertyEditor | null;
    /** Makes blocks editable by double-clicking them. */
    blockEditing?: BlockEditing | null;
    /**
     * When the next `doc` is an edit of the current one: update it in place,
     * keeping every section mounted and the scroll position, instead of
     * remounting at `initialAnchor`.
     */
    keepPosition?: boolean;
    onpropertiestoggle?: (open: boolean) => void;
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
    propertiesOpen = true,
    propertyEditor = null,
    blockEditing = null,
    keepPosition = false,
    onpropertiestoggle,
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

  const keys = $derived(blockKeys(doc));
  onDestroy(() => cancelIdle?.());
  let shownPath: string | null = null;

  // (Re)start mounting whenever the document changes, or for an edit of the
  // same document, update it in place.
  $effect(() => {
    const d = doc;
    const anchor = initialAnchor;
    // `untrack` keeps the mount work from subscribing this effect to
    // `mounted` (which it writes) and to `keepPosition`.
    untrack(() => {
      if (keepPosition && shownPath === d.path) {
        updateInPlace(d);
      } else {
        void startMount(d, anchor);
      }
      shownPath = d.path;
    });
  });

  /**
   * Shows an edited version of the same document: every section mounted, so
   * unchanged blocks keep their DOM (they're keyed by content), and the
   * browser's scroll anchoring keeps the reader's place.
   */
  function updateInPlace(d: DocumentModel): void {
    cancelIdle?.();
    cancelIdle = null;
    mounted = d.sections.map(() => true);
    void tick().then(noteScrolled);
  }

  // The block being edited, by id, and the source the editor started from.
  let editing = $state<{ id: number; source: string } | null>(null);

  async function onDoubleClick(event: MouseEvent): Promise<void> {
    if (!blockEditing || editing) return;
    const target = event.target as Element | null;
    // Not in the properties panel or endnotes, and not on a control.
    if (target?.closest(".sk-properties, .sk-endnotes, input, button, a")) return;
    const el = target?.closest<HTMLElement>(".sk-section > .sk-block[data-block]");
    const block = el ? doc.blocks[Number(el.dataset.block)] : undefined;
    if (!block || block.kind.type === "footnoteDefinition") return;
    // The double-click selected a word; the editor replaces the text anyway.
    window.getSelection()?.removeAllRanges();
    try {
      const source = await blockEditing.load(block);
      editing = { id: block.id, source };
    } catch {
      // The document changed underneath; leave it in reading view.
    }
  }

  async function finishEdit(block: Block, text: string): Promise<void> {
    const original = editing?.source ?? "";
    editing = null;
    if (text === original) return;
    await blockEditing?.commit(block, original, text);
  }

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
    // The very top of the first block is the top of the document, with the
    // properties panel above it in view. (A position inside the panel is
    // captured as exactly this, since the panel isn't a block.)
    if (block.id === doc.blocks[0]?.id && anchor.fraction === 0 && offsetY === 0) {
      scroller.scrollTop = 0;
      noteScrolled();
      return;
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

  let column: HTMLElement | undefined = $state();

  /**
   * Smoothly scales the text column with a CSS transform (no reflow), keeping
   * the point `offsetY` px below the viewport top in place. Used to animate
   * keyboard zoom; the real font size is applied afterwards.
   */
  export function previewScale(scale: number, offsetY: number, durationMs: number): Promise<void> {
    if (!scroller || !column) return Promise.resolve();
    // The fixed point, in the column's own (unscaled) coordinates. While a
    // preview is already running, the column is scaled around that same
    // point, and measuring it now would be off by the current scale: keep it.
    if (!column.style.transform) {
      const originY = scroller.getBoundingClientRect().top + offsetY - column.getBoundingClientRect().top;
      column.style.transformOrigin = `50% ${originY}px`;
      // Make the browser register "no transform" before the transition
      // starts, or it could animate from a just-cleared earlier scale.
      void column.offsetWidth;
    }
    column.style.transition = `transform ${durationMs}ms cubic-bezier(0.2, 0.7, 0.3, 1)`;
    column.style.transform = `scale(${scale})`;
    return new Promise((resolve) => window.setTimeout(resolve, durationMs));
  }

  export function clearPreview(): void {
    if (!column) return;
    column.style.transition = "none";
    column.style.transform = "";
    column.style.transformOrigin = "";
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
    popover = null;
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
  function assetImages(node: HTMLElement, params: { token: number; html: string }) {
    // Runs again whenever the token or the block's HTML changes: a live
    // reload keeps the element but replaces its HTML, dropping the srcs.
    const apply = ({ token }: { token: number; html: string }) => {
      for (const img of node.querySelectorAll<HTMLImageElement>("img[data-asset]")) {
        img.src = assetUrl(token, Number(img.dataset.asset));
      }
    };
    apply(params);
    return { update: apply };
  }

  let endnotes: HTMLElement | undefined = $state();

  /** Scrolls to the element with HTML id `id` (a heading slug or footnote). */
  export async function scrollToId(id: string): Promise<void> {
    // Footnotes are shown in the endnotes section, outside the blocks.
    const endnote = endnotes?.querySelector(`[id="${CSS.escape(contentId(id))}"]`);
    if (endnote) {
      endnote.scrollIntoView({ block: "start" });
      noteScrolled();
      return;
    }
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

  // Footnote popovers: hovering or focusing a reference shows its note.
  let popover = $state<{ html: string; x: number; y: number; below: boolean } | null>(null);
  let popoverEl: HTMLElement | undefined = $state();
  let hideTimer = 0;
  /** How long the pointer may be away from reference and popover before it hides. */
  const POPOVER_HIDE_MS = 250;

  /** The footnote a reference link points at (`href="#fn-<name>"`). */
  function footnoteFor(link: Element): { html: string } | null {
    const href = link.getAttribute("href") ?? "";
    if (!href.startsWith("#fn-")) return null;
    const name = decodeURIComponent(href.slice("#fn-".length));
    return doc.footnotes.find((f) => f.name === name) ?? null;
  }

  async function showPopover(link: Element): Promise<void> {
    const note = footnoteFor(link);
    if (!note) return;
    window.clearTimeout(hideTimer);
    const r = link.getBoundingClientRect();
    // Measure at the left edge, where the viewport doesn't squeeze it.
    popover = { html: note.html, x: 0, y: r.bottom + 6, below: true };
    await tick();
    if (!popoverEl || !popover) return;
    // Center it on the reference, kept on screen; flip above if there's no room below.
    const width = popoverEl.offsetWidth;
    const height = popoverEl.offsetHeight;
    const x = Math.min(Math.max(8, r.left + r.width / 2 - width / 2), window.innerWidth - width - 8);
    const below = r.bottom + 6 + height <= window.innerHeight - 8;
    popover = { ...popover, x, y: below ? r.bottom + 6 : Math.max(8, r.top - 6 - height), below };
  }

  function scheduleHide(): void {
    window.clearTimeout(hideTimer);
    hideTimer = window.setTimeout(() => (popover = null), POPOVER_HIDE_MS);
  }

  function onPointerOver(event: PointerEvent): void {
    const target = event.target as Element | null;
    if (target?.closest(".sk-footnote-popover")) {
      window.clearTimeout(hideTimer);
      return;
    }
    const ref = target?.closest("a[data-footnote-ref]");
    if (ref) void showPopover(ref);
  }

  function onPointerOut(event: PointerEvent): void {
    const target = event.target as Element | null;
    if (target?.closest("a[data-footnote-ref], .sk-footnote-popover")) scheduleHide();
  }

  function onFocusIn(event: FocusEvent): void {
    const ref = (event.target as Element | null)?.closest("a[data-footnote-ref]");
    if (ref) void showPopover(ref);
  }

  async function onClick(event: MouseEvent): Promise<void> {
    popover = null;
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
<div
  class="sk-scroller"
  bind:this={scroller}
  onscroll={onScroll}
  onclick={onClick}
  ondblclick={onDoubleClick}
  onpointerover={onPointerOver}
  onpointerout={onPointerOut}
  onfocusin={onFocusIn}
  onfocusout={scheduleHide}
  role="document"
>
  <article bind:this={column} class="sk-column" class:fill={fillWindow} style:font-size={fontSize === null ? null : `${fontSize}px`}>
    {#if doc.frontMatter}
      <PropertiesPanel
        frontMatter={doc.frontMatter}
        open={propertiesOpen}
        ontoggle={(open) => onpropertiestoggle?.(open)}
        onlink={(href) => onlink?.(href)}
        editor={propertyEditor}
      />
    {/if}
    {#each doc.sections as section, i (section.id)}
      {#if mounted[i]}
        <section
          class="sk-section"
          data-section={section.id}
          style:contain-intrinsic-size={`auto ${heights[i] ?? 0}px`}
        >
          {#each sectionBlocks(doc, section) as block (keys[block.id])}
            {#if editing?.id === block.id}
              <div class="sk-block sk-editing" data-block={block.id}>
                <BlockEditor source={editing.source} oncommit={(text) => finishEdit(block, text)} />
              </div>
            {:else if block.kind.type === "footnoteDefinition"}
              <!-- Shown in the endnotes instead; the empty block keeps its place for
                   scroll anchoring and, later, editing. -->
              <div class="sk-block sk-footnote-def" data-block={block.id}></div>
            {:else}
              <!-- Block.html is sanitized by core (ammonia); it's the only HTML we insert. -->
              <div class="sk-block" data-block={block.id} use:assetImages={{ token: assetToken, html: block.html }} use:richContent={block.html}
                >{@html block.html}</div
              >
            {/if}
          {/each}
        </section>
      {:else}
        <section class="sk-section sk-placeholder" data-section={section.id} style:height={`${heights[i] ?? 0}px`}></section>
      {/if}
    {/each}
    {#if doc.footnotes.length > 0}
      <section class="sk-block sk-endnotes" bind:this={endnotes} aria-label="Notes">
        <h2>Notes</h2>
        <ol>
          {#each doc.footnotes as note (note.name)}
            <!-- Footnote html is sanitized by core, like Block.html. -->
            <li
              id={contentId(`fn-${note.name}`)}
              value={note.number}
              use:assetImages={{ token: assetToken, html: note.html }}
              use:richContent={note.html}
            >
              {@html note.html}<a class="sk-backref" href={`#fnref-${note.name}`} aria-label={`Back to reference ${note.number}`}>↩</a>
            </li>
          {/each}
        </ol>
      </section>
    {/if}
    {#if popover}
      <div
        class="sk-block sk-footnote-popover"
        bind:this={popoverEl}
        role="tooltip"
        style:left={`${popover.x}px`}
        style:top={`${popover.y}px`}
        use:assetImages={{ token: assetToken, html: popover.html }}
        use:richContent={popover.html}
      >
        {@html popover.html}
      </div>
    {/if}
  </article>
</div>

<style>
  .sk-scroller {
    /* Lets layout classes size things to the reading area (`100cqw`). */
    container-type: inline-size;
    height: 100%;
    overflow-y: auto;
    /* The zoom preview briefly scales the column wider than the view. */
    overflow-x: hidden;
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
