<script lang="ts">
  // The keyboard shortcut reference (F1 or Ctrl+/), read from
  // `SHORTCUT_GROUPS`. Escape, the × button, or clicking outside closes it.
  import { trapFocus } from "../lib/focus";
  import { onMount } from "svelte";
  import { keyParts, SHORTCUT_GROUPS } from "./shortcuts";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();
  let closeButton: HTMLButtonElement | undefined = $state();

  onMount(() => closeButton?.focus());

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onclose();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onkeydown={onKeydown} onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="panel" use:trapFocus role="dialog" aria-modal="true" aria-labelledby="sk-shortcuts-title">
    <header>
      <h2 id="sk-shortcuts-title">Keyboard shortcuts</h2>
      <button bind:this={closeButton} class="close" aria-label="Close" title="Close (Esc)" onclick={onclose}>×</button>
    </header>
    <div class="groups">
      {#each SHORTCUT_GROUPS as group (group.title)}
        <section>
          <h3>{group.title}{#if group.where}<span class="where">, {group.where}</span>{/if}</h3>
          <dl>
            {#each group.shortcuts as shortcut (shortcut.action)}
              <dt>
                {#each shortcut.keys as keys, i (keys)}
                  {#if i > 0}<span class="or">or</span>{/if}
                  <span class="combo">
                    {#each keyParts(keys) as part, j (j)}
                      {#if j > 0}<span class="plus">+</span>{/if}<kbd>{part}</kbd>
                    {/each}
                  </span>
                {/each}
              </dt>
              <dd>{shortcut.action}</dd>
            {/each}
          </dl>
        </section>
      {/each}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 6vh 1rem 0;
    z-index: 11;
  }

  .panel {
    width: min(66em, 100%);
    max-height: 86vh;
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
    display: flex;
    align-items: center;
    padding: 0.8rem 1rem 0.5rem;
    border-bottom: 1px solid var(--sk-color-border);
  }

  h2 {
    flex: 1;
    margin: 0;
    font-size: 1rem;
    font-family: var(--sk-font-ui);
  }

  .close {
    font: inherit;
    font-size: 1.1rem;
    line-height: 1;
    color: var(--sk-color-muted);
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 0.1em 0.4em;
    cursor: pointer;
  }

  .close:hover,
  .close:focus-visible {
    color: var(--sk-color-foreground);
    border-color: var(--sk-color-border);
  }

  .groups {
    overflow-y: auto;
    padding: 0.4rem 1rem 1rem;
    /* In em of the panel's UI text: rem is the theme's reading size. */
    columns: 2 27em;
    column-gap: 2.5em;
  }

  section {
    break-inside: avoid;
    padding-top: 0.6rem;
  }

  h3 {
    margin: 0 0 0.35rem;
    font-size: 0.8rem;
    font-family: var(--sk-font-ui);
    color: var(--sk-color-accent);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .where {
    text-transform: none;
    letter-spacing: normal;
    color: var(--sk-color-muted);
    font-weight: normal;
  }

  dl {
    display: grid;
    /* The same key column in every group, so the actions line up. */
    grid-template-columns: 15em 1fr;
    gap: 0.3rem 0.9rem;
    margin: 0;
    align-items: baseline;
  }

  dt {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.25rem;
  }

  dd {
    margin: 0;
  }

  .combo {
    white-space: nowrap;
  }

  .or,
  .plus {
    color: var(--sk-color-muted);
    font-size: 0.75rem;
  }

  .plus {
    margin: 0 0.1rem;
  }

  kbd {
    font-family: var(--sk-font-ui);
    font-size: 0.75rem;
    padding: 0.05rem 0.35rem;
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-bottom-width: 2px;
    border-radius: 4px;
    white-space: nowrap;
  }
</style>
