<script lang="ts" module>
  export type UnsavedChoice = "save" | "discard" | "cancel";
</script>

<script lang="ts">
  // Asked before closing the window or leaving a document with unsaved
  // changes (DESIGN.md §9.3): save, discard, or stay.
  import { onMount } from "svelte";

  interface Props {
    /** The document's title, for the question. */
    title: string;
    /** What happens next, e.g. "closing" or "opening another document". */
    action: string;
    onchoose: (choice: UnsavedChoice) => void;
  }

  let { title, action, onchoose }: Props = $props();
  let saveButton: HTMLButtonElement | undefined = $state();

  onMount(() => saveButton?.focus());

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onchoose("cancel");
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="sk-backdrop" role="presentation" onkeydown={onKeydown}>
  <div class="sk-dialog" role="alertdialog" aria-modal="true" aria-labelledby="sk-unsaved-title" aria-describedby="sk-unsaved-text">
    <h2 id="sk-unsaved-title">Save changes to “{title}”?</h2>
    <p id="sk-unsaved-text">You have unsaved changes. If you don't save them before {action}, they'll be lost.</p>
    <div class="sk-buttons">
      <button class="primary" bind:this={saveButton} onclick={() => onchoose("save")}>Save</button>
      <button onclick={() => onchoose("discard")}>Don't save</button>
      <button onclick={() => onchoose("cancel")}>Cancel</button>
    </div>
  </div>
</div>

<style>
  .sk-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--sk-color-background) 55%, transparent);
  }

  .sk-dialog {
    max-width: min(28rem, calc(100vw - 2rem));
    padding: 1.2rem 1.4rem;
    font-family: var(--sk-font-ui);
    font-size: 0.95rem;
    color: var(--sk-color-foreground);
    background: var(--sk-color-ui-background, var(--sk-color-background));
    border: 1px solid var(--sk-color-border);
    border-radius: 8px;
    box-shadow: 0 10px 40px color-mix(in srgb, var(--sk-color-foreground) 25%, transparent);
  }

  h2 {
    margin: 0 0 0.5rem;
    font-size: 1.05rem;
  }

  p {
    margin: 0 0 1.1rem;
    color: var(--sk-color-muted);
  }

  .sk-buttons {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  button {
    font: inherit;
    padding: 0.35rem 0.9rem;
    border-radius: 5px;
    border: 1px solid var(--sk-color-border);
    background: var(--sk-color-background);
    color: var(--sk-color-foreground);
    cursor: pointer;
  }

  button.primary {
    background: var(--sk-color-accent);
    border-color: var(--sk-color-accent);
    color: var(--sk-color-background);
  }

  button:focus-visible {
    outline: 2px solid var(--sk-color-accent);
    outline-offset: 2px;
  }
</style>
