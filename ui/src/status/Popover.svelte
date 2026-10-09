<script lang="ts">
  // A small themed panel that opens above the status bar (text size, flavor).
  // Closes on Escape or a click outside it; the status bar button that opens
  // it is marked `data-popover-toggle="<toggle>"` and toggles it instead.
  import type { Snippet } from "svelte";

  interface Props {
    /** Accessible name of the panel. */
    label: string;
    /** Matches the `data-popover-toggle` value of the button that opens this panel. */
    toggle: string;
    width?: string;
    onclose: () => void;
    children: Snippet;
  }

  let { label, toggle, width = "17rem", onclose, children }: Props = $props();

  let panel: HTMLDivElement | undefined = $state();

  $effect(() => {
    // Focus the panel so Escape works at once; let a child take focus if it asks.
    if (panel && !panel.contains(document.activeElement)) panel.focus();
  });

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
    }
  }

  function onWindowPointerDown(event: PointerEvent): void {
    const target = event.target as Element | null;
    if (!target || panel?.contains(target)) return;
    if (target.closest(`[data-popover-toggle="${toggle}"]`)) return;
    onclose();
  }
</script>

<svelte:window onpointerdown={onWindowPointerDown} />

<div
  class="popover"
  bind:this={panel}
  tabindex="-1"
  role="dialog"
  aria-label={label}
  style:width
  onkeydown={onKeydown}
>
  {@render children()}
</div>

<style>
  .popover {
    position: fixed;
    right: 0.75rem;
    bottom: 2.2rem;
    z-index: 10;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    background: var(--sk-color-ui-background);
    color: var(--sk-color-foreground);
    border: 1px solid var(--sk-color-border);
    border-radius: 8px;
    box-shadow: 0 8px 28px color-mix(in srgb, var(--sk-color-foreground) 22%, transparent);
    font-family: var(--sk-font-ui);
    font-size: 0.8rem;
    outline: none;
  }
</style>
