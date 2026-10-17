<script lang="ts">
  // Shown when Scrald starts without a document (DESIGN.md §11): an Open
  // button, pinned and recent documents with a swatch of the theme each was
  // last shown in, and anything else the app puts in `children` (recovered
  // changes). Files can also be dropped anywhere on the window.
  import type { Snippet } from "svelte";
  import type { RecentDocument, ThemeSummary } from "../lib/types";
  import { displayTitle, lastOpenedText, splitPinned } from "./recent";

  interface Props {
    documents: RecentDocument[];
    themes: ThemeSummary[];
    /** A line about the launch (say, a file that wasn't found). */
    message: string | null;
    onpick: () => void;
    onopen: (doc: RecentDocument) => void;
    onpin: (doc: RecentDocument, pinned: boolean) => void;
    onforget: (doc: RecentDocument) => void;
    children?: Snippet;
  }

  let { documents, themes, message, onpick, onopen, onpin, onforget, children }: Props = $props();

  const groups = $derived(splitPinned(documents));
  const themesById = $derived(new Map(themes.map((t) => [t.id, t])));
  // Read once: "5 minutes ago" needn't tick while the screen is open.
  const now = Date.now();
</script>

{#snippet list(docs: RecentDocument[])}
  <ul>
    {#each docs as doc (doc.path)}
      {@const theme = doc.theme ? themesById.get(doc.theme) : undefined}
      <li class:missing={!doc.exists}>
        <button
          class="open"
          onclick={() => onopen(doc)}
          disabled={!doc.exists}
          title={doc.exists ? doc.path : `Not found: ${doc.path}`}
        >
          <span
            class="swatch"
            class:themed={theme !== undefined}
            style:--swatch-bg={theme?.background}
            style:--swatch-fg={theme?.foreground}
            style:--swatch-accent={theme?.accent}
            title={theme?.name}
            aria-hidden="true">Aa</span
          >
          <span class="text">
            <span class="title">{displayTitle(doc)}</span>
            <span class="path">{doc.path}</span>
            <span class="when">
              {#if doc.exists}
                Opened {lastOpenedText(doc.lastOpened, now)}
              {:else}
                Not found. It may have been moved, renamed, or deleted.
              {/if}
            </span>
          </span>
        </button>
        <span class="actions">
          <button
            class="icon"
            onclick={() => onpin(doc, !doc.pinned)}
            aria-pressed={doc.pinned}
            title={doc.pinned ? "Unpin" : "Pin to the top"}
            aria-label={doc.pinned ? `Unpin ${displayTitle(doc)}` : `Pin ${displayTitle(doc)}`}
            >{doc.pinned ? "★" : "☆"}</button
          >
          <button
            class="icon"
            onclick={() => onforget(doc)}
            title="Remove from the list (Scrald forgets its reading position and settings)"
            aria-label={`Remove ${displayTitle(doc)} from the list`}>×</button
          >
        </span>
      </li>
    {/each}
  </ul>
{/snippet}

<div class="sk-start">
  <div class="column">
    <h1>Scrald</h1>
    <div class="open-row">
      <button class="primary" onclick={onpick} title="Open a Markdown file (Ctrl+O)">Open…</button>
      <span class="hint">or drop a Markdown file anywhere on this window</span>
    </div>
    {#if message}<p class="message">{message}</p>{/if}

    {@render children?.()}

    {#if groups.pinned.length > 0}
      <section>
        <h2>Pinned</h2>
        {@render list(groups.pinned)}
      </section>
    {/if}
    <section>
      <h2>Recent</h2>
      {#if groups.recent.length > 0}
        {@render list(groups.recent)}
      {:else}
        <p class="empty">
          {groups.pinned.length > 0 ? "No other documents yet." : "Documents you open will be listed here."}
        </p>
      {/if}
    </section>
  </div>
</div>

<style>
  /* Scrolls at the window's full width; the column inside is centered. */
  .sk-start {
    overflow-y: auto;
    font-family: var(--sk-font-ui);
    font-size: 0.9rem;
    color: var(--sk-color-foreground);
  }

  .column {
    max-width: 46rem;
    margin: 0 auto;
    padding: 3rem 1.5rem;
  }

  h1 {
    font-family: var(--sk-font-heading);
    font-size: 2rem;
    margin: 0 0 1rem;
  }

  h2 {
    font-size: 1.1rem;
    margin: 2rem 0 0.5rem;
  }

  .open-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5em 1em;
  }

  .hint,
  .message,
  .empty,
  .path,
  .when {
    color: var(--sk-color-muted);
  }

  .message {
    overflow-wrap: anywhere;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    align-items: center;
    gap: 0.5em;
    border-top: 1px solid var(--sk-color-border);
  }

  li:last-child {
    border-bottom: 1px solid var(--sk-color-border);
  }

  button {
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.3em 1em;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    border-color: var(--sk-color-accent);
  }

  button.primary {
    border-color: var(--sk-color-accent);
  }

  button.open {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.9em;
    text-align: left;
    border: 1px solid transparent;
    background: none;
    padding: 0.6em 0.5em;
  }

  button.open:disabled {
    cursor: default;
  }

  li.missing .title,
  li.missing .swatch {
    opacity: 0.55;
  }

  .swatch {
    flex: none;
    width: 2.6em;
    height: 2.6em;
    display: grid;
    place-items: center;
    box-sizing: border-box;
    border-radius: 4px;
    border: 1px solid var(--sk-color-border);
    border-bottom-width: 4px;
    font-family: var(--sk-font-heading);
    font-size: 0.95em;
    background: var(--sk-color-code-bg);
    color: var(--sk-color-muted);
  }

  .swatch.themed {
    background: var(--swatch-bg);
    color: var(--swatch-fg);
    border-bottom-color: var(--swatch-accent);
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .title {
    font-weight: 600;
    font-size: 1.05em;
  }

  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    gap: 0.3em;
    padding-right: 0.3em;
  }

  button.icon {
    padding: 0.1em 0.5em;
    border-color: transparent;
    background: none;
    color: var(--sk-color-muted);
    font-size: 1.1em;
  }

  button.icon[aria-pressed="true"] {
    color: var(--sk-color-accent);
  }

  button.icon:hover {
    color: var(--sk-color-foreground);
  }
</style>
