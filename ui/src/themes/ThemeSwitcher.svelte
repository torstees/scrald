<script lang="ts">
  // Theme switcher (DESIGN.md §7.6): Ctrl+T. Arrow keys preview themes live
  // on the current document; Enter keeps the selection for this document,
  // Escape puts the original theme back.
  import { onMount } from "svelte";
  import type { ResolvedTheme, ThemeSummary } from "../lib/types";
  import { describeSource, stepIndex } from "./apply";

  interface Props {
    themes: ThemeSummary[];
    /** The document's theme when the switcher opened. */
    current: ResolvedTheme;
    folder: string | null;
    onpreview: (id: string) => void;
    /** Keep `id` for this document. */
    onchoose: (id: string) => void;
    onmakedefault: (id: string) => void;
    /** Clear the per-document choice. */
    onreset: () => void;
    onduplicate: (id: string) => void;
    /** Write `id` into the document's front matter (`scrald-theme`). */
    onsavetodocument: (id: string) => void;
    /** Close without choosing (the caller reverts the preview). */
    oncancel: () => void;
  }

  let {
    themes,
    current,
    folder,
    onpreview,
    onchoose,
    onmakedefault,
    onreset,
    onduplicate,
    onsavetodocument,
    oncancel,
  }: Props = $props();

  const usable = $derived(themes.filter((t) => t.error === null));
  const broken = $derived(themes.filter((t) => t.error !== null));
  let selected = $state(0);
  let list: HTMLUListElement | undefined = $state();

  onMount(() => {
    selected = Math.max(
      0,
      usable.findIndex((t) => t.id === current.id),
    );
    list?.focus();
  });

  function select(index: number): void {
    selected = index;
    const theme = usable[index];
    if (theme) onpreview(theme.id);
    list?.querySelector<HTMLElement>(`[data-index="${index}"]`)?.scrollIntoView({ block: "nearest" });
  }

  function chosenId(): string | null {
    return usable[selected]?.id ?? null;
  }

  /** Runs `action` with the selected theme's id, if there is a selection. */
  function act(action: (id: string) => void): void {
    const id = chosenId();
    if (id) action(id);
  }

  function onKeydown(event: KeyboardEvent): void {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        select(stepIndex(selected, 1, usable.length));
        break;
      case "ArrowUp":
        event.preventDefault();
        select(stepIndex(selected, -1, usable.length));
        break;
      case "Enter":
        event.preventDefault();
        act(onchoose);
        break;
      case "Escape":
        event.preventDefault();
        oncancel();
        break;
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onkeydown={onKeydown} onclick={(e) => e.target === e.currentTarget && oncancel()}>
  <div class="panel" role="dialog" aria-modal="true" aria-label="Choose a theme">
    <header>
      <h2>Theme</h2>
      <p class="source">Current: {current.id}, {describeSource(current.source)}</p>
    </header>

    <ul bind:this={list} tabindex="-1" role="listbox" aria-label="Themes">
      {#each usable as theme, index (theme.id)}
        <li
          data-index={index}
          role="option"
          aria-selected={index === selected}
          class:selected={index === selected}
          onclick={() => select(index)}
          ondblclick={() => onchoose(theme.id)}
          onkeydown={() => {}}
        >
          <span class="swatch" style:background={theme.background} style:color={theme.foreground}>
            Aa<span class="dot" style:background={theme.accent}></span>
          </span>
          <span class="name">{theme.name}</span>
          <span class="meta">{theme.bundled ? "built in" : "yours"} · {theme.appearance}</span>
        </li>
      {/each}
    </ul>

    {#if broken.length > 0}
      <details class="broken">
        <summary>{broken.length} theme{broken.length === 1 ? "" : "s"} could not be loaded</summary>
        {#each broken as theme (theme.id)}
          <p><strong>{theme.id}</strong></p>
          <pre>{theme.error}</pre>
        {/each}
      </details>
    {/if}

    <footer>
      <div class="actions">
        <button class="primary" onclick={() => act(onchoose)}>Use for this document</button>
        <button onclick={() => act(onsavetodocument)} title="Writes scrald-theme into the front matter (save with Ctrl+S)"
          >Save to document</button
        >
        <button onclick={() => act(onmakedefault)}>Make default</button>
        <button onclick={() => act(onduplicate)}>Duplicate to customize</button>
        {#if current.source === "document"}
          <button onclick={onreset}>Reset to document default</button>
        {/if}
      </div>
      <p class="hint">↑↓ preview · Enter use · Esc cancel{folder ? ` · Your themes: ${folder}` : ""}</p>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: color-mix(in srgb, var(--sk-color-background) 40%, transparent);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 8vh;
    z-index: 10;
  }

  .panel {
    width: min(34rem, 92vw);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    background: var(--sk-color-ui-background);
    color: var(--sk-color-foreground);
    border: 1px solid var(--sk-color-border);
    border-radius: 8px;
    box-shadow: 0 12px 40px color-mix(in srgb, var(--sk-color-foreground) 25%, transparent);
    font-family: var(--sk-font-ui);
    font-size: 0.85rem;
  }

  header {
    padding: 0.9rem 1rem 0.4rem;
  }

  h2 {
    margin: 0;
    font-size: 1rem;
    font-family: var(--sk-font-ui);
  }

  .source {
    margin: 0.2rem 0 0;
    color: var(--sk-color-muted);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0.25rem 0.5rem;
    overflow-y: auto;
    outline: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.35rem 0.5rem;
    border-radius: 6px;
    cursor: pointer;
  }

  li.selected {
    background: var(--sk-color-selection);
  }

  .swatch {
    position: relative;
    width: 2.6rem;
    height: 1.8rem;
    border-radius: 4px;
    border: 1px solid var(--sk-color-border);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: Georgia, serif;
  }

  .dot {
    position: absolute;
    right: 3px;
    bottom: 3px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .name {
    flex: 1;
    font-weight: 500;
  }

  .meta {
    color: var(--sk-color-muted);
    font-size: 0.75rem;
  }

  .broken {
    margin: 0 1rem;
    color: var(--sk-color-muted);
  }

  .broken pre {
    white-space: pre-wrap;
    font-size: 0.75rem;
    margin: 0 0 0.5rem;
  }

  footer {
    padding: 0.6rem 1rem 0.8rem;
    border-top: 1px solid var(--sk-color-border);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  button {
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.25rem 0.6rem;
    cursor: pointer;
  }

  button.primary {
    border-color: var(--sk-color-accent);
    color: var(--sk-color-accent);
  }

  .hint {
    margin: 0.5rem 0 0;
    color: var(--sk-color-muted);
    font-size: 0.72rem;
    overflow-wrap: anywhere;
  }
</style>
