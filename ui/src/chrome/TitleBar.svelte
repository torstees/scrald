<script lang="ts">
  // The window title, "• Title — ~/notes/a.md". When the path doesn't fit,
  // it gives up its first folders, so the file name stays.
  //
  // - macOS: drawn here, in the title bar (Tauri's "Overlay" style keeps the
  //   traffic lights; tauri.macos.conf.json hides the system's title).
  //   Hovering the path shows it in full, and clicking copies it. The bar
  //   drags the window, and double-clicking it zooms, as a title bar does.
  // - Windows and Linux: the system draws the title bar and would cut the
  //   end, so this sets it to a title already shortened to fit, measured in
  //   the system's title font and kept up to date as the window resizes.
  //   The status bar shows the full path there instead.
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onWindowTitle, windowTitle } from "../lib/commands";
  import { IS_MAC } from "../lib/platform";
  import type { WindowTitle } from "../lib/types";
  import { fitPath, fittedTitle, titleName } from "./titlebar";

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

  const name = $derived(title ? titleName(title) : "");

  let unlisten: (() => void) | null = null;
  let observer: ResizeObserver | null = null;
  let canvas: CanvasRenderingContext2D | null = null;

  onMount(() => {
    void onWindowTitle((next) => {
      // The app has just set the system title in full; set it again.
      nativeShown = "";
      title = next;
    }).then((stop) => (unlisten = stop));
    void windowTitle().then((current) => (title ??= current));
    // The bar's width changes with the window; the hidden copy of the
    // title changes width when a theme changes the interface font.
    observer = new ResizeObserver(() => fit());
    if (bar) observer.observe(bar);
    if (measureEl) observer.observe(measureEl);
    if (!IS_MAC) window.addEventListener("resize", fitNative);
  });

  onDestroy(() => {
    unlisten?.();
    observer?.disconnect();
    window.removeEventListener("resize", fitNative);
  });

  $effect(() => {
    void title;
    if (IS_MAC) fit();
    else fitNative();
  });

  /** Width of `text` in a CSS `font`. */
  function measure(text: string, font: string): number {
    canvas ??= document.createElement("canvas").getContext("2d");
    if (!canvas) return 0;
    canvas.font = font;
    return canvas.measureText(text).width;
  }

  /** Width of `text` in the element's font. */
  function widthOf(text: string, el: HTMLElement): number {
    return measure(text, getComputedStyle(el).font);
  }

  // The system title bar, in CSS pixels (which scale with the display, as
  // the title bar does). Windows 11: Segoe UI at 9 pt beside the icon, with
  // three 46 px buttons at the right. Linux: GNOME centers a bold title
  // between the window buttons; fonts vary by desktop, so the guess is
  // generous. Both leave a margin, since a title cut at the end is what
  // this avoids.
  const NATIVE = navigator.userAgent.includes("Windows")
    ? { font: '12px "Segoe UI Variable Text", "Segoe UI", sans-serif', reserved: 3 * 46 + 48, margin: 1.1 }
    : { font: 'bold 15px Cantarell, Ubuntu, "Noto Sans", sans-serif', reserved: 2 * 100, margin: 1.15 };

  let nativeShown = "";

  /** Sets the system title to one that fits the window (Windows, Linux). */
  function fitNative(): void {
    if (!title) return;
    const room = window.innerWidth - NATIVE.reserved;
    const text = fittedTitle(title, (t) => measure(t, NATIVE.font) * NATIVE.margin <= room);
    if (text === nativeShown) return;
    nativeShown = text;
    void getCurrentWindow()
      .setTitle(text)
      .catch(() => {});
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

{#if IS_MAC}
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
{/if}

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
