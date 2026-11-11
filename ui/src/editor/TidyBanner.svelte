<script lang="ts">
  // Shown over raw mode's merge view after tidying hard-wrapped text (#219):
  // the text before on the left, the tidied text on the right. The arrows
  // put back an original line break; Done keeps the right side.
  import { keyLabel } from "../lib/platform";
  import { tidySummary } from "../status/text";

  interface Props {
    joined: number;
    split: number;
    busy: boolean;
    ondone: () => void;
    onundo: () => void;
  }

  let { joined, split, busy, ondone, onundo }: Props = $props();
</script>

<div class="sk-tidy" role="status" aria-busy={busy}>
  <p>
    {tidySummary(joined, split)}. <strong>Left</strong> is the text before (read-only), <strong>right</strong> is the tidied
    text. The arrows put back a change you don't want.
  </p>
  <div class="sk-tidy-actions">
    <button class="primary" onclick={ondone} disabled={busy} title={keyLabel("The tidied text stays as unsaved changes; Ctrl+S saves it")}>
      Done
    </button>
    <button onclick={onundo} disabled={busy} title="Puts the text back as it was before tidying">Undo tidy</button>
  </div>
</div>

<style>
  .sk-tidy {
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

  .sk-tidy-actions {
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
