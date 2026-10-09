<script lang="ts">
  // Table of contents sidebar (DESIGN.md §10): click to jump, current
  // section highlighted, collapsible entries, and a depth limit for long
  // documents such as novels.
  import { SvelteSet } from "svelte/reactivity";
  import type { TocEntry } from "../lib/types";
  import { hasChildren, nearestVisible, visibleEntries } from "./tree";

  interface Props {
    toc: TocEntry[];
    /** Index into `toc` of the current section's heading, if any. */
    current: number | null;
    onselect: (index: number) => void;
    /** Show outline numbers (when the theme numbers headings). */
    numbering?: boolean;
  }

  let { toc, current, onselect, numbering = false }: Props = $props();

  let maxLevel = $state(6);
  const collapsed = new SvelteSet<number>();
  const visible = $derived(visibleEntries(toc, maxLevel, collapsed));
  const highlighted = $derived(nearestVisible(visible, current));
  const deepest = $derived(toc.reduce((max, e) => Math.max(max, e.level), 1));
  const shallowest = $derived(toc.reduce((min, e) => Math.min(min, e.level), 6));

  function toggle(index: number): void {
    if (collapsed.has(index)) collapsed.delete(index);
    else collapsed.add(index);
  }
</script>

<nav class="sk-toc" aria-label="Table of contents">
  <header>
    <span class="title">Contents</span>
    {#if deepest > shallowest}
      <label>
        Depth
        <select bind:value={maxLevel}>
          {#each Array.from({ length: deepest - shallowest + 1 }, (_, k) => shallowest + k) as level (level)}
            <option value={level}>H{level}</option>
          {/each}
          <option value={6}>All</option>
        </select>
      </label>
    {/if}
  </header>

  {#if toc.length === 0}
    <p class="empty">No headings</p>
  {:else}
    <ul>
      {#each visible as index (index)}
        {@const entry = toc[index]}
        {#if entry}
          <li style:padding-left={`${(entry.level - shallowest) * 0.9}rem`} class:current={index === highlighted}>
            {#if hasChildren(toc, index) && entry.level < maxLevel}
              <button
                class="twisty"
                aria-label={collapsed.has(index) ? "Expand" : "Collapse"}
                aria-expanded={!collapsed.has(index)}
                onclick={() => toggle(index)}>{collapsed.has(index) ? "▸" : "▾"}</button
              >
            {:else}
              <span class="twisty-space"></span>
            {/if}
            <button class="entry" onclick={() => onselect(index)} title={entry.text}
              >{#if numbering && entry.number}<span class="number">{entry.number}</span>{/if}{entry.text}</button
            >
          </li>
        {/if}
      {/each}
    </ul>
  {/if}
</nav>

<style>
  .sk-toc {
    height: 100%;
    overflow-y: auto;
    box-sizing: border-box;
    padding: 0.75rem 0.5rem 2rem;
    font-family: var(--sk-font-ui);
    font-size: 0.8rem;
    background: var(--sk-color-ui-background);
    border-right: 1px solid var(--sk-color-border);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0.25rem 0.5rem;
    color: var(--sk-color-muted);
  }

  .title {
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.7rem;
  }

  select {
    font: inherit;
    color: inherit;
    background: var(--sk-color-ui-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 3px;
  }

  /* The drop-down list is drawn by the webview; set its colors explicitly. */
  option {
    background: var(--sk-color-ui-background);
    color: var(--sk-color-foreground);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    align-items: baseline;
    border-radius: 4px;
  }

  li.current {
    background: var(--sk-color-selection);
  }

  li.current .entry {
    color: var(--sk-color-foreground);
    font-weight: 600;
  }

  button {
    font: inherit;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--sk-color-muted);
  }

  .twisty,
  .twisty-space {
    flex: 0 0 1rem;
    padding: 0;
  }

  .entry {
    flex: 1;
    text-align: left;
    padding: 0.2rem 0.25rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .number {
    color: var(--sk-color-muted);
    margin-right: 0.4em;
    font-variant-numeric: tabular-nums;
  }

  .entry:hover {
    color: var(--sk-color-foreground);
  }

  .empty {
    color: var(--sk-color-muted);
    padding: 0 0.25rem;
  }
</style>
