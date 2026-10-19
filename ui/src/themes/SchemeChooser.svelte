<script lang="ts">
  // Chooses which color schemes to import from a terminal theme file that
  // holds several, such as a Windows Terminal settings.json (DESIGN.md §7.3).
  import { trapFocus } from "../lib/focus";
  import { onMount } from "svelte";

  interface Props {
    /** The file, for the heading. */
    path: string;
    names: string[];
    /** Import the schemes with these names. */
    onimport: (names: string[]) => void;
    oncancel: () => void;
  }

  let { path, names, onimport, oncancel }: Props = $props();

  // Indexes, since two schemes in a file may share a name.
  let chosen = $state<number[]>([]);
  let list: HTMLUListElement | undefined = $state();
  const fileName = $derived(path.split(/[\\/]/).pop() ?? path);

  onMount(() => list?.querySelector<HTMLInputElement>("input")?.focus());

  function toggle(index: number): void {
    chosen = chosen.includes(index) ? chosen.filter((i) => i !== index) : [...chosen, index];
  }

  function importChosen(): void {
    const picked = names.filter((_, i) => chosen.includes(i));
    if (picked.length > 0) onimport(picked);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      oncancel();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onkeydown={onKeydown} onclick={(e) => e.target === e.currentTarget && oncancel()}>
  <div class="panel" use:trapFocus role="dialog" aria-modal="true" aria-label="Choose color schemes to import">
    <header>
      <h2>Import color schemes</h2>
      <p class="source" title={path}>{names.length} schemes in {fileName}</p>
    </header>
    <ul bind:this={list}>
      {#each names as name, index (index)}
        <li>
          <label>
            <input type="checkbox" checked={chosen.includes(index)} onchange={() => toggle(index)} />
            {name}
          </label>
        </li>
      {/each}
    </ul>
    <footer>
      <button class="primary" disabled={chosen.length === 0} onclick={importChosen}>
        Import{chosen.length > 1 ? ` ${chosen.length}` : ""}
      </button>
      <button onclick={() => (chosen = names.map((_, i) => i))}>Select all</button>
      <button onclick={oncancel}>Cancel</button>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    z-index: 11;
  }

  .panel {
    width: min(26rem, 90vw);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    background: var(--sk-color-ui-background);
    color: var(--sk-color-foreground);
    border: 1px solid var(--sk-color-border);
    border-radius: 8px;
    box-shadow: 0 12px 40px color-mix(in srgb, var(--sk-color-foreground) 25%, transparent);
    font-family: var(--sk-font-ui);
    font-size: 0.85rem;
  }

  header {
    padding: 0.9rem 1rem 0.4rem;
  }

  h2 {
    margin: 0;
    font-size: 1rem;
    font-family: var(--sk-font-ui);
  }

  .source {
    margin: 0.2rem 0 0;
    color: var(--sk-color-muted);
    overflow-wrap: anywhere;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0.25rem 1rem;
    overflow-y: auto;
  }

  li label {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.25rem 0;
    cursor: pointer;
  }

  input {
    accent-color: var(--sk-color-accent);
  }

  footer {
    display: flex;
    gap: 0.4rem;
    padding: 0.6rem 1rem 0.8rem;
    border-top: 1px solid var(--sk-color-border);
  }

  button {
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.25rem 0.6rem;
    cursor: pointer;
  }

  button.primary {
    border-color: var(--sk-color-accent);
    color: var(--sk-color-accent);
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
