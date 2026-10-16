<script lang="ts">
  // The banner shown when the file changes on disk while there are unsaved
  // edits (DESIGN.md §9.3): keep mine, load the disk version, or compare the
  // two side by side in raw mode.

  interface Props {
    /** "changed": someone else edited the file; "missing": it's gone. */
    state: "changed" | "missing";
    /** True while the compare view is showing. */
    comparing: boolean;
    busy: boolean;
    onkeep: () => void;
    onload: () => void;
    oncompare: () => void;
  }

  let { state, comparing, busy, onkeep, onload, oncompare }: Props = $props();
</script>

<div class="sk-conflict" role="alert" aria-busy={busy}>
  {#if state === "missing"}
    <p>The file was moved or deleted. Your changes are still here; <code>Ctrl+S</code> writes the file again.</p>
    <div class="sk-conflict-actions">
      <button onclick={onkeep} disabled={busy}>OK</button>
    </div>
  {:else if comparing}
    <p>
      Comparing with the file on disk: <strong>left</strong> is the disk version (read-only),
      <strong>right</strong> is yours. The arrows copy a change from disk into yours.
    </p>
    <div class="sk-conflict-actions">
      <button class="primary" onclick={onkeep} disabled={busy}>Done: keep my version</button>
      <button onclick={onload} disabled={busy}>Use the disk version</button>
    </div>
  {:else}
    <p>This file changed on disk while you have unsaved changes.</p>
    <div class="sk-conflict-actions">
      <button class="primary" onclick={onkeep} disabled={busy} title="Saving will replace the disk version">
        Keep mine
      </button>
      <button onclick={onload} disabled={busy} title="Your changes can be brought back with Ctrl+Z">
        Load disk version
      </button>
      <button onclick={oncompare} disabled={busy}>Compare</button>
    </div>
  {/if}
</div>

<style>
  .sk-conflict {
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

  code {
    font-family: var(--sk-font-mono);
  }

  .sk-conflict-actions {
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
