<script lang="ts">
  // Shown when Scrald starts without a document: every set of unsaved
  // changes left behind by a run that stopped before saving them
  // (DESIGN.md §9.3). Part of the start screen.
  import type { RecoveredChanges } from "../lib/types";
  import { savedWhen } from "./recovered";

  interface Props {
    items: RecoveredChanges[];
    busy: boolean;
    onrestore: (item: RecoveredChanges) => void;
    oncopy: (item: RecoveredChanges) => void;
    ondiscard: (item: RecoveredChanges) => void;
  }

  let { items, busy, onrestore, oncopy, ondiscard }: Props = $props();
</script>

<section class="sk-recovered" aria-busy={busy}>
  <h2>Unsaved changes found</h2>
  <p class="intro">Scrald closed before these changes were saved.</p>
  <ul>
    {#each items as item (item.id)}
      <li>
        <div class="title">{item.title}</div>
        <div class="path">{item.path}</div>
        <div class="when">
          From {savedWhen(item.savedAt)}.
          {#if item.fileMissing}
            The file is missing; copy the text to keep it.
          {:else if item.fileChanged}
            The file has changed since then.
          {/if}
        </div>
        <div class="actions">
          <button class="primary" onclick={() => onrestore(item)} disabled={busy || item.fileMissing}>
            Restore
          </button>
          <button onclick={() => oncopy(item)} disabled={busy}>Copy text</button>
          <button onclick={() => ondiscard(item)} disabled={busy} title="Deletes the recovered changes">
            Discard
          </button>
        </div>
      </li>
    {/each}
  </ul>
</section>

<style>
  .sk-recovered {
    font-family: var(--sk-font-ui);
    font-size: 0.9rem;
    color: var(--sk-color-foreground);
  }

  h2 {
    font-size: 1.1rem;
    margin: 2rem 0 0.25rem;
  }

  .intro,
  .path,
  .when {
    color: var(--sk-color-muted);
  }

  .intro {
    margin: 0 0 0.75rem;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    padding: 0.6em 0;
    border-top: 1px solid var(--sk-color-border);
  }

  .title {
    font-weight: 600;
  }

  .path {
    overflow-wrap: anywhere;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4em;
    margin-top: 0.4em;
  }

  button {
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.2em 0.8em;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    border-color: var(--sk-color-accent);
  }

  button.primary {
    border-color: var(--sk-color-accent);
  }

  button:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
