<script lang="ts">
  // The window title on macOS, drawn in the title bar (Tauri's "Overlay"
  // style keeps the traffic lights; tauri.macos.conf.json hides the system's
  // title). "• Title — ~/notes/a.md": the path gives up its first folders
  // when it doesn't fit, so the file name stays. Hovering the path shows it
  // in full, and clicking copies it. The bar drags the window, and
  // double-clicking it zooms, as a title bar does.
  import { onDestroy, onMount } from "svelte";
  import { onWindowTitle, windowTitle } from "../lib/commands";
  import type { WindowTitle } from "../lib/types";
  import { fitPath } from "./titlebar";

  interface Props {
    /** Called after the path is copied, to say so. */
    oncopied?: (path: string) => void;
  }

  let { oncopied }: Props = $props();

  let title = $state<WindowTitle | null>(null);
  let shownPath = $state("");
  let bar: HTMLElement | undefined = $state();
  let nameEl: HTMLElement | undefined = $state();
  let pathEl: HTMLElement | undefined = $state();
  let measureEl: HTMLElement | undefined = $state();

  const name = $derived(title ? `${title.dirty ? "• " : ""}${title.title}` : "");

  let unlisten: (() => void) | null = null;
  let observer: ResizeObserver | null = null;
  let canvas: CanvasRenderingContext2D | null = null;

  onMount(() => {
    void onWindowTitle((next) => (title = next)).then((stop) => (unlisten = stop));
    void windowTitle().then((current) => (title ??= current));
    // The bar's width changes with the window; the hidden copy of the
    // title changes width when a theme changes the interface font.
    observer = new ResizeObserver(() => fit());
    if (bar) observer.observe(bar);
    if (measureEl) observer.observe(measureEl);
  });

  onDestroy(() => {
    unlisten?.();
    observer?.disconnect();
  });

  $effect(() => {
    void title;
    fit();
  });

  /** Width of `text` in the element's font. */
  function widthOf(text: string, el: HTMLElement): number {
    canvas ??= document.createElement("canvas").getContext("2d");
    if (!canvas) return 0;
    canvas.font = getComputedStyle(el).font;
    return canvas.measureText(text).width;
  }

  /** Shortens the path to the room the bar has beside the title. */
  function fit(): void {
    const path = title?.path;
    if (!path || !bar || !nameEl || !pathEl) {
      shownPath = path ?? "";
      return;
    }
    const style = getComputedStyle(bar);
    const inner = bar.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
    const room = inner - widthOf(name, nameEl) - widthOf(" — ", nameEl);
    shownPath = fitPath(path, (text) => widthOf(text, pathEl!) <= room);
  }

  async function copyPath(): Promise<void> {
    const full = title?.fullPath;
    if (!full) return;
    try {
      await navigator.clipboard.writeText(full);
      oncopied?.(full);
    } catch {
      // The clipboard refused; the tooltip still shows the path.
    }
  }
</script>

<header class="sk-titlebar" bind:this={bar} data-tauri-drag-region>
  {#if title}
    <span class="name" bind:this={nameEl} data-tauri-drag-region>{name}</span>
    {#if title.path}
      <span class="dash" data-tauri-drag-region>&nbsp;—&nbsp;</span>
      <button class="path" bind:this={pathEl} title={`${title.fullPath ?? title.path}\nClick to copy`} onclick={copyPath}
        >{shownPath}</button
      >
    {/if}
    <span class="measure" bind:this={measureEl} aria-hidden="true">{name} — {title.path ?? ""}</span>
  {/if}
</header>

<style>
  .sk-titlebar {
    position: fixed;
    inset: 0 0 auto 0;
    z-index: 5;
    height: var(--sk-titlebar-height);
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    /* Room for the traffic lights, on both sides so the title stays centered. */
    padding: 0 80px;
    font-family: var(--sk-font-ui);
    font-size: 13px;
    color: var(--sk-color-foreground);
    background: var(--sk-color-ui-background);
    border-bottom: 1px solid var(--sk-color-border);
    white-space: nowrap;
    user-select: none;
    -webkit-user-select: none;
    cursor: default;
  }

  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 600;
  }

  .dash {
    flex: none;
    color: var(--sk-color-muted);
  }

  .path {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font: inherit;
    color: var(--sk-color-muted);
    background: none;
    border: none;
    padding: 0;
    cursor: default;
  }

  .path:hover {
    color: var(--sk-color-foreground);
  }

  .measure {
    position: absolute;
    visibility: hidden;
    pointer-events: none;
  }

  @media print {
    .sk-titlebar {
      display: none;
    }
  }
</style>
