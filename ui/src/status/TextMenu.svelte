<script lang="ts">
  // The line-ending menu (#219, #220), opened from the status bar: convert
  // line endings, remove a BOM, and tidy hard-wrapped text. Each is an
  // ordinary edit, undone with Ctrl+Z and written only on Ctrl+S.
  import { keyLabel } from "../lib/platform";
  import type { LineEnding } from "../lib/types";
  import Popover from "./Popover.svelte";
  import { conversions } from "./text";

  interface Props {
    lineEnding: LineEnding;
    mixed: boolean;
    hasBom: boolean;
    /** In raw mode, tidying takes the selection (or the whole text without one). */
    raw: boolean;
    onconvert: (to: LineEnding) => void;
    onremovebom: () => void;
    ontidy: () => void;
    onclose: () => void;
  }

  let { lineEnding, mixed, hasBom, raw, onconvert, onremovebom, ontidy, onclose }: Props = $props();

  const NAMES: Record<LineEnding, string> = { lf: "LF (macOS, Linux)", crlf: "CRLF (Windows)" };

  let list: HTMLDivElement | undefined = $state();

  $effect(() => {
    list?.querySelector<HTMLButtonElement>("button")?.focus();
  });

  function run(action: () => void): void {
    onclose();
    action();
  }

  /** Up and Down move between items, like a native menu. */
  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const buttons = Array.from(list?.querySelectorAll<HTMLButtonElement>("button") ?? []);
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = (at + (event.key === "ArrowDown" ? 1 : buttons.length - 1)) % buttons.length;
    buttons[next]?.focus();
  }
</script>

<Popover label="Line endings and text" toggle="text" width="17rem" {onclose}>
  <span class="label">
    {#if mixed}
      Lines end in both CRLF and LF.
    {:else}
      Lines end in {lineEnding === "crlf" ? "CRLF" : "LF"}.
    {/if}
    {#if hasBom}The file starts with a byte order mark (BOM).{/if}
  </span>
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div class="items" role="menu" aria-label="Line endings and text" bind:this={list} onkeydown={onKeydown}>
    {#each conversions(lineEnding, mixed) as to (to)}
      <button role="menuitem" onclick={() => run(() => onconvert(to))}>Convert to {NAMES[to]}</button>
    {/each}
    {#if hasBom}
      <button role="menuitem" onclick={() => run(onremovebom)}>Remove BOM</button>
    {/if}
    <hr />
    <button
      role="menuitem"
      onclick={() => run(ontidy)}
      title={raw
        ? "Joins lines broken mid-paragraph in the selection, or in the whole document if nothing is selected"
        : "Joins lines broken mid-paragraph, and separates paragraphs that run together"}
    >
      Tidy hard-wrapped text… <span class="key">{keyLabel("Alt+Q")}</span>
    </button>
  </div>
  <p class="hint">Changes are unsaved until you save; Undo takes them back.</p>
</Popover>

<style>
  .label {
    color: var(--sk-color-muted);
    white-space: normal;
  }

  .items {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  hr {
    width: 100%;
    margin: 0.2rem 0;
    border: none;
    border-top: 1px solid var(--sk-color-border);
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
