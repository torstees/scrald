<script lang="ts">
  // Offered when a document opens and Scrald has unsaved changes to it from
  // a run that stopped before they were saved (DESIGN.md §9.3). Compare
  // restores them into raw mode's merge view, beside the file.
  import type { RecoveredChanges } from "../lib/types";
  import { savedWhen } from "./recovered";

  interface Props {
    changes: RecoveredChanges;
    /** True while the merge view shows the restored changes beside the file. */
    comparing: boolean;
    busy: boolean;
    onrestore: () => void;
    ondiscard: () => void;
    oncompare: () => void;
    /** Comparing: keep the text on the right as unsaved changes. */
    ondone: () => void;
    /** Comparing: go back to the file's text. */
    onusefile: () => void;
  }

  let { changes, comparing, busy, onrestore, ondiscard, oncompare, ondone, onusefile }: Props = $props();
</script>

<div class="sk-recovery" role="alert" aria-busy={busy}>
  {#if comparing}
    <p>
      Comparing the recovered changes with the file: <strong>left</strong> is the file (read-only),
      <strong>right</strong> is the recovered text. The arrows copy a change from the file.
    </p>
    <div class="sk-recovery-actions">
      <button class="primary" onclick={ondone} disabled={busy} title="The text on the right stays as unsaved changes">
        Done
      </button>
      <button onclick={onusefile} disabled={busy} title="Ctrl+Z brings the recovered text back">
        Use the file
      </button>
    </div>
  {:else}
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
      <button onclick={oncompare} disabled={busy} title="Shows the recovered text beside the file, to take changes from both">
        Compare
      </button>
      <button onclick={ondiscard} disabled={busy} title="Deletes the recovered changes">Discard</button>
    </div>
  {/if}
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
