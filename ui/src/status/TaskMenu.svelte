<script lang="ts">
  // The task-list menu (#218), opened from the status bar's "3 / 12 done":
  // hide or show the document's checked items. Display only; the file
  // doesn't change.
  import { keyLabel } from "../lib/platform";
  import Popover from "./Popover.svelte";

  interface Props {
    done: number;
    total: number;
    hideChecked: boolean;
    ontoggle: () => void;
    onclose: () => void;
  }

  let { done, total, hideChecked, ontoggle, onclose }: Props = $props();

  let item: HTMLButtonElement | undefined = $state();

  $effect(() => {
    item?.focus();
  });
</script>

<Popover label="Task items" toggle="tasks" width="16rem" {onclose}>
  <span class="label">{done} of {total} {total === 1 ? "task" : "tasks"} checked.</span>
  <div class="items" role="menu" aria-label="Task items">
    <button
      role="menuitemcheckbox"
      aria-checked={hideChecked}
      bind:this={item}
      onclick={() => {
        onclose();
        ontoggle();
      }}
    >
      <span><span class="tick" aria-hidden="true">{hideChecked ? "✓" : ""}</span>Hide checked items</span>
      <span class="key">{keyLabel("Ctrl+Shift+H")}</span>
    </button>
  </div>
  <p class="hint">Click a checkbox to check or uncheck it. Changes are unsaved until you save; Undo takes them back.</p>
</Popover>

<style>
  .label {
    color: var(--sk-color-muted);
    white-space: normal;
  }

  .items {
    display: flex;
    flex-direction: column;
  }

  button {
    font: inherit;
    display: flex;
    justify-content: space-between;
    gap: 1em;
    text-align: left;
    color: var(--sk-color-foreground);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 0.3rem 0.55rem;
    cursor: pointer;
  }

  button:hover,
  button:focus-visible {
    background: var(--sk-color-background);
    border-color: var(--sk-color-border);
  }

  .tick {
    display: inline-block;
    width: 1.2em;
  }

  .key {
    color: var(--sk-color-muted);
  }

  .hint {
    margin: 0;
    color: var(--sk-color-muted);
    font-size: 0.72rem;
    white-space: normal;
  }
</style>
