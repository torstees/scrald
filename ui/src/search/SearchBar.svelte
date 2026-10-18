<script lang="ts">
  // The find bar over the reader (DESIGN.md §10): `Ctrl+F` opens it, Enter
  // and Shift+Enter (or F3 and Shift+F3) step through the matches, and
  // Escape closes it.

  interface Props {
    query: string;
    matchCase: boolean;
    wholeWord: boolean;
    /** "3 of 120", "No matches", or "". */
    label: string;
    /** Whether there are matches to step through. */
    hasMatches: boolean;
    onchange: (query: string, matchCase: boolean, wholeWord: boolean) => void;
    onnext: () => void;
    onprevious: () => void;
    onclose: () => void;
  }

  let { query, matchCase, wholeWord, label, hasMatches, onchange, onnext, onprevious, onclose }: Props = $props();

  let input: HTMLInputElement | undefined = $state();

  /** Focuses the field with its text selected, ready to type over. */
  export function focus(): void {
    input?.focus();
    input?.select();
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Enter") {
      event.preventDefault();
      if (event.shiftKey) onprevious();
      else onnext();
    } else if (event.key === "Escape") {
      event.preventDefault();
      onclose();
    }
  }
</script>

<div class="sk-search" role="search">
  <input
    bind:this={input}
    type="text"
    value={query}
    placeholder="Find in document"
    aria-label="Find in document"
    spellcheck="false"
    oninput={(e) => onchange(e.currentTarget.value, matchCase, wholeWord)}
    onkeydown={onKeydown}
  />
  <span class="sk-search-count" aria-live="polite">{label}</span>
  <button
    class="toggle"
    class:on={matchCase}
    aria-pressed={matchCase}
    title="Match case"
    onclick={() => onchange(query, !matchCase, wholeWord)}>Aa</button
  >
  <button
    class="toggle word"
    class:on={wholeWord}
    aria-pressed={wholeWord}
    title="Whole word"
    onclick={() => onchange(query, matchCase, !wholeWord)}>ab</button
  >
  <button disabled={!hasMatches} title="Previous match (Shift+Enter)" aria-label="Previous match" onclick={onprevious}
    >↑</button
  >
  <button disabled={!hasMatches} title="Next match (Enter)" aria-label="Next match" onclick={onnext}>↓</button>
  <button title="Close (Esc)" aria-label="Close search" onclick={onclose}>×</button>
</div>

<style>
  .sk-search {
    position: absolute;
    top: 0.5rem;
    right: 1.75rem;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.3rem 0.4rem;
    background: var(--sk-color-ui-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 6px;
    box-shadow: 0 2px 8px rgb(0 0 0 / 0.15);
    color: var(--sk-color-foreground);
    font-family: var(--sk-font-ui);
    font-size: 0.85rem;
    max-width: calc(100% - 2.5rem);
    box-sizing: border-box;
  }

  input {
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.2em 0.5em;
    width: 14em;
    min-width: 6em;
    flex: 1 1 auto;
  }

  input:focus {
    outline: none;
    border-color: var(--sk-color-accent);
  }

  .sk-search-count {
    color: var(--sk-color-muted);
    min-width: 5.5em;
    text-align: center;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  button {
    font: inherit;
    color: var(--sk-color-foreground);
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 0.1em 0.45em;
    min-width: 1.8em;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    border-color: var(--sk-color-border);
  }

  button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .toggle.on {
    border-color: var(--sk-color-accent);
    color: var(--sk-color-accent);
  }

  .toggle.word {
    text-decoration: underline;
    text-underline-offset: 0.2em;
  }
</style>
