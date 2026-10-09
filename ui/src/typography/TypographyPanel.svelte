<script lang="ts">
  // Text sizing and zoom controls (DESIGN.md §7.4), opened from the status bar.
  import { formatZoom, MAX_ZOOM, MIN_ZOOM, type TextSizing } from "./typography";

  interface Props {
    mode: TextSizing;
    zoom: number;
    fillWindow: boolean;
    onmode: (mode: TextSizing) => void;
    onzoomin: () => void;
    onzoomout: () => void;
    onzoomreset: () => void;
    onfillwindow: (fill: boolean) => void;
    onmakedefault: () => void;
    onclose: () => void;
  }

  let {
    mode,
    zoom,
    fillWindow,
    onmode,
    onzoomin,
    onzoomout,
    onzoomreset,
    onfillwindow,
    onmakedefault,
    onclose,
  }: Props = $props();

  let panel: HTMLDivElement | undefined = $state();

  $effect(() => {
    panel?.focus();
  });

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
    }
  }

  /**
   * Close on a click anywhere outside the panel. The status bar button that
   * opens the panel is exempt (marked `data-typography-toggle`); it toggles.
   */
  function onWindowPointerDown(event: PointerEvent): void {
    const target = event.target as Element | null;
    if (!target || panel?.contains(target) || target.closest("[data-typography-toggle]")) return;
    onclose();
  }
</script>

<svelte:window onpointerdown={onWindowPointerDown} />

<div
  class="panel"
  bind:this={panel}
  tabindex="-1"
  role="dialog"
  aria-label="Text size and zoom"
  onkeydown={onKeydown}
>
  <div class="row">
    <span class="label">Text size</span>
    <div class="segmented" role="radiogroup" aria-label="Text sizing mode">
      <button role="radio" aria-checked={mode === "fixed"} class:on={mode === "fixed"} onclick={() => onmode("fixed")}
        >Fixed</button
      >
      <button role="radio" aria-checked={mode === "fit"} class:on={mode === "fit"} onclick={() => onmode("fit")}
        >Fit to window</button
      >
    </div>
  </div>

  <div class="row">
    <span class="label">Zoom</span>
    <div class="zoom">
      <button onclick={onzoomout} disabled={zoom <= MIN_ZOOM} aria-label="Zoom out" title="Zoom out (Ctrl+−)">−</button>
      <button class="value" onclick={onzoomreset} title="Reset to 100% (Ctrl+0)">{formatZoom(zoom)}</button>
      <button onclick={onzoomin} disabled={zoom >= MAX_ZOOM} aria-label="Zoom in" title="Zoom in (Ctrl+=)">+</button>
    </div>
  </div>

  <label class="row check">
    <input type="checkbox" checked={fillWindow} onchange={(e) => onfillwindow(e.currentTarget.checked)} />
    Fill window (ignore line length)
  </label>

  <button class="default" onclick={onmakedefault}>Make this the default</button>
  <p class="hint">Applies to documents without their own setting.</p>
</div>

<style>
  .panel {
    position: fixed;
    right: 0.75rem;
    bottom: 2.2rem;
    z-index: 10;
    width: 17rem;
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

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .label {
    color: var(--sk-color-muted);
  }

  button {
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.2rem 0.55rem;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .segmented {
    display: flex;
  }

  .segmented button {
    border-radius: 0;
  }

  .segmented button:first-child {
    border-radius: 4px 0 0 4px;
  }

  .segmented button:last-child {
    border-radius: 0 4px 4px 0;
    border-left: none;
  }

  .segmented button.on {
    background: var(--sk-color-selection);
    font-weight: 600;
  }

  .zoom {
    display: flex;
    gap: 0.25rem;
  }

  .zoom .value {
    min-width: 3.6rem;
    font-variant-numeric: tabular-nums;
  }

  .check {
    justify-content: flex-start;
    cursor: pointer;
  }

  .default {
    border-color: var(--sk-color-accent);
    color: var(--sk-color-accent);
  }

  .hint {
    margin: -0.3rem 0 0;
    color: var(--sk-color-muted);
    font-size: 0.72rem;
  }
</style>
