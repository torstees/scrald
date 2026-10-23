<script lang="ts">
  // "Contents | Properties": the left sidebar's tabs when the properties
  // panel shares it (DESIGN.md §8.2).
  import { keyLabel } from "../lib/platform";
  import { tick } from "svelte";
  import type { Panel } from "./layout";

  interface Props {
    tab: Panel;
    ontab: (tab: Panel) => void;
  }

  let { tab, ontab }: Props = $props();

  const TABS: { id: Panel; label: string; keys: string }[] = [
    { id: "contents", label: "Contents", keys: "Ctrl+\\" },
    { id: "properties", label: "Properties", keys: "Ctrl+Shift+\\" },
  ];

  /**
   * Left and right arrows move between the tabs (wrapping around), and Home
   * and End go to the first and last, as in any tab list.
   */
  async function onKeydown(event: KeyboardEvent): Promise<void> {
    const current = TABS.findIndex((t) => t.id === tab);
    let index: number;
    if (event.key === "ArrowLeft") index = (current - 1 + TABS.length) % TABS.length;
    else if (event.key === "ArrowRight") index = (current + 1) % TABS.length;
    else if (event.key === "Home") index = 0;
    else if (event.key === "End") index = TABS.length - 1;
    else return;
    const next = TABS[index];
    if (!next) return;
    event.preventDefault();
    ontab(next.id);
    // Each tab is a different sidebar component, so choosing one replaces this
    // tab list with a new one. Focus the chosen tab there once it's rendered;
    // the buttons here are about to leave the page.
    await tick();
    document.querySelector<HTMLButtonElement>('.sk-tabs [role="tab"][aria-selected="true"]')?.focus();
  }
</script>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div class="sk-tabs" role="tablist" aria-label="Sidebar" onkeydown={onKeydown}>
  {#each TABS as t (t.id)}
    <button
      role="tab"
      aria-selected={tab === t.id}
      tabindex={tab === t.id ? 0 : -1}
      title={`${t.label} (${keyLabel(t.keys)})`}
      onclick={() => ontab(t.id)}>{t.label}</button
    >
  {/each}
</div>

<style>
  .sk-tabs {
    display: flex;
    gap: 0.2rem;
  }

  button {
    font: inherit;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.7rem;
    color: var(--sk-color-muted);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 0.1rem 0.25rem;
    cursor: pointer;
  }

  button:hover {
    color: var(--sk-color-foreground);
  }

  button[aria-selected="true"] {
    color: var(--sk-color-foreground);
    border-bottom-color: var(--sk-color-accent);
  }
</style>
