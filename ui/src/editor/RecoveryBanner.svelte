<script lang="ts">
  // Offered when a document opens and Scrald has unsaved changes to it from
  // a run that stopped before they were saved (DESIGN.md §9.3).
  import type { RecoveredChanges } from "../lib/types";
  import { savedWhen } from "./recovered";

  interface Props {
    changes: RecoveredChanges;
    busy: boolean;
    onrestore: () => void;
    ondiscard: () => void;
  }

  let { changes, busy, onrestore, ondiscard }: Props = $props();
</script>

<div class="sk-recovery" role="alert" aria-busy={busy}>
  <p>
    Found unsaved changes to this document from {savedWhen(changes.savedAt)}. Scrald closed before they were saved.
    {#if changes.fileChanged}
      The file has changed since then, so saving them would replace that newer version.
    {/if}
  </p>
  <div class="sk-recovery-actions">
    <button
      class="primary"
      onclick={onrestore}
      disabled={busy}
      title="They become unsaved changes; Ctrl+Z goes back to the file"
    >
      Restore
    </button>
    <button onclick={ondiscard} disabled={busy} title="Deletes the recovered changes">Discard</button>
  </div>
</div>

<style>
  .sk-recovery {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5em 1em;
    padding: 0.5em 1em;
    background: var(--sk-color-highlight-bg);
    border-bottom: 1px solid var(--sk-color-border);
    color: var(--sk-color-foreground);
    font-family: var(--sk-font-ui);
    font-size: 0.9rem;
  }

  p {
    margin: 0;
    flex: 1 1 20em;
  }

  .sk-recovery-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4em;
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
