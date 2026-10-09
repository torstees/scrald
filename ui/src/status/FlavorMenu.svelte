<script lang="ts">
  // The Markdown flavor menu (DESIGN.md §5.2), opened from the status bar.
  import type { Flavor, FlavorSource } from "../lib/types";
  import { autoLabel, FLAVOR_NAMES, switcherValue } from "./flavor";
  import Popover from "./Popover.svelte";

  interface Props {
    flavor: Flavor;
    flavorSource: FlavorSource;
    /** A flavor the user picked, or null for automatic. */
    onchoose: (flavor: Flavor | null) => void;
    onclose: () => void;
  }

  let { flavor, flavorSource, onchoose, onclose }: Props = $props();

  const current = $derived(switcherValue(flavor, flavorSource));
  // When a flavor is chosen, "Auto" can't say what detection would pick
  // without re-parsing, so it stays generic.
  const options = $derived<{ value: Flavor | "auto"; label: string }[]>([
    { value: "auto", label: current === "auto" ? autoLabel(flavor, flavorSource) : "Auto" },
    ...Object.entries(FLAVOR_NAMES).map(([value, label]) => ({ value: value as Flavor, label })),
  ]);

  let list: HTMLDivElement | undefined = $state();

  $effect(() => {
    list?.querySelector<HTMLButtonElement>('[aria-checked="true"]')?.focus();
  });

  function choose(value: Flavor | "auto"): void {
    onclose();
    if (value !== current) onchoose(value === "auto" ? null : value);
  }

  /** Up and Down move between options, like a native menu. */
  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const buttons = Array.from(list?.querySelectorAll<HTMLButtonElement>("button") ?? []);
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = (at + (event.key === "ArrowDown" ? 1 : buttons.length - 1)) % buttons.length;
    buttons[next]?.focus();
  }
</script>

<Popover label="Markdown flavor" toggle="flavor" width="15rem" {onclose}>
  <span class="label">Markdown flavor</span>
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div class="options" role="radiogroup" aria-label="Markdown flavor" bind:this={list} onkeydown={onKeydown}>
    {#each options as option (option.value)}
      <button
        role="radio"
        aria-checked={option.value === current}
        class:on={option.value === current}
        onclick={() => choose(option.value)}>{option.label}</button
      >
    {/each}
  </div>
  <p class="hint">Auto uses front matter, a folder's .scrald.toml, or detection.</p>
</Popover>

<style>
  .label {
    color: var(--sk-color-muted);
  }

  .options {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  button {
    font: inherit;
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
    outline: none;
  }

  button.on {
    background: var(--sk-color-selection);
    font-weight: 600;
  }

  .hint {
    margin: 0;
    color: var(--sk-color-muted);
    font-size: 0.72rem;
  }
</style>
