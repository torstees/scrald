<script lang="ts">
  // Print (DESIGN.md §10), opened with Ctrl+P or the status bar's 🖶: how
  // the pages are colored and whether link addresses are printed. Print
  // hands over to the system's print dialog, which also saves as PDF.
  import { trapFocus } from "../lib/focus";
  import { onMount } from "svelte";
  import type { PrintColors, PrintOptions } from "./print";

  interface Props {
    /** What the panel starts with. */
    options: PrintOptions;
    /** The document's theme, for the colors' labels. */
    themeName: string;
    onprint: (options: PrintOptions) => void;
    oncancel: () => void;
  }

  let { options, themeName, onprint, oncancel }: Props = $props();

  // svelte-ignore state_referenced_locally
  let colors = $state<PrintColors>(options.colors);
  // svelte-ignore state_referenced_locally
  let linkAddresses = $state(options.linkAddresses);
  let printButton: HTMLButtonElement | undefined = $state();

  onMount(() => printButton?.focus());

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      oncancel();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="sk-backdrop" role="presentation" onkeydown={onKeydown} onclick={(e) => e.target === e.currentTarget && oncancel()}>
  <div class="sk-dialog" use:trapFocus role="dialog" aria-modal="true" aria-labelledby="sk-print-title">
    <h2 id="sk-print-title">Print</h2>
    <fieldset>
      <legend>Colors</legend>
      <label>
        <input type="radio" name="sk-print-colors" value="theme" checked={colors === "theme"} onchange={() => (colors = "theme")} />
        <span>
          Theme colors
          <small>{themeName} as on screen, page color included. Suits a PDF.</small>
        </span>
      </label>
      <label>
        <input type="radio" name="sk-print-colors" value="paper" checked={colors === "paper"} onchange={() => (colors = "paper")} />
        <span>
          White paper
          <small>{themeName}'s fonts and layout in Scrald Light's colors on a white page. Saves ink.</small>
        </span>
      </label>
    </fieldset>
    <label class="check">
      <input type="checkbox" checked={linkAddresses} onchange={(e) => (linkAddresses = e.currentTarget.checked)} />
      <span>
        Show link addresses
        <small>Prints each web link's address after it, for readers who can't click.</small>
      </span>
    </label>
    <p class="hint">Next comes the system's print dialog, where you choose the printer or Save as PDF.</p>
    <div class="sk-buttons">
      <button class="primary" bind:this={printButton} onclick={() => onprint({ colors, linkAddresses })}>Print…</button>
      <button onclick={oncancel}>Cancel</button>
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
    max-width: min(26rem, calc(100vw - 2rem));
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
    margin: 0 0 0.6rem;
    font-size: 1.05rem;
  }

  fieldset {
    margin: 0 0 0.6rem;
    padding: 0;
    border: none;
  }

  legend {
    padding: 0;
    margin-bottom: 0.3rem;
    font-weight: 600;
  }

  label {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    margin: 0.35rem 0;
    cursor: pointer;
  }

  label.check {
    margin-top: 0.8rem;
  }

  input {
    accent-color: var(--sk-color-accent);
    margin: 0;
  }

  small {
    display: block;
    color: var(--sk-color-muted);
    font-size: 0.85em;
    line-height: 1.35;
  }

  .hint {
    margin: 0.9rem 0 1rem;
    color: var(--sk-color-muted);
    font-size: 0.85em;
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
</style>
