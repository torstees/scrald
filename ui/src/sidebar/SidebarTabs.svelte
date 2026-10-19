<script lang="ts">
  // "Contents | Properties": the left sidebar's tabs when the properties
  // panel shares it (DESIGN.md §8.2).
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

  /** Left and right arrows move between the tabs, as in any tab list. */
  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const next = tab === "contents" ? "properties" : "contents";
    ontab(next);
    const list = (event.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>("[role=tab]");
    list[next === "contents" ? 0 : 1]?.focus();
  }
</script>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div class="sk-tabs" role="tablist" aria-label="Sidebar" onkeydown={onKeydown}>
  {#each TABS as t (t.id)}
    <button
      role="tab"
      aria-selected={tab === t.id}
      tabindex={tab === t.id ? 0 : -1}
      title={`${t.label} (${t.keys})`}
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
