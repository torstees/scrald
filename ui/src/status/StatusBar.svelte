<script lang="ts">
  import type { Flavor, FlavorSource } from "../lib/types";
  import { FLAVOR_NAMES } from "./flavor";
  import FlavorMenu from "./FlavorMenu.svelte";
  import { formatProgress, formatTimeLeft } from "./progress";
  import { formatReadingTime, formatWordCount } from "./reading";

  interface Props {
    wordCount: number;
    /** Title of the section at the top of the viewport. */
    section: string | null;
    /** A short-lived message (link problems and the like), shown instead of the section. */
    notice: string | null;
    canGoBack: boolean;
    canGoForward: boolean;
    onback: () => void;
    onforward: () => void;
    /** Remote images in the document, and whether they're loading. */
    remoteImages: number;
    remoteImagesAllowed: boolean;
    /** Loads (true) or blocks (false) the document's remote images. */
    onremoteimages: (allowed: boolean) => void;
    /** Name of the document's theme; clicking it opens the switcher. */
    themeName: string | null;
    onthemeclick: () => void;
    tocVisible: boolean;
    ontoggletoc: () => void;
    /**
     * Whether the right properties sidebar is shown, or null for no button
     * (the panel is at the top, or a tab at the left).
     */
    propertiesShown?: boolean | null;
    ontoggleproperties?: () => void;
    flavor: Flavor;
    flavorSource: FlavorSource;
    /** A flavor the user picked, or null for automatic. */
    onflavor: (flavor: Flavor | null) => void;
    /** Text sizing and zoom summary; clicking it opens the typography panel. */
    typography: string;
    ontypographyclick: () => void;
    /** Whether autosave is on; shown only then, and clicking opens settings. */
    autosave: boolean;
    /** Opens the settings dialog. */
    onsettings: () => void;
    /** Share of the document read, 0..1 (DESIGN.md §10). */
    progress: number;
    /** Opens the keyboard shortcut reference. */
    onshortcuts: () => void;
    onprint: () => void;
  }

  let {
    wordCount,
    section,
    notice,
    canGoBack,
    canGoForward,
    onback,
    onforward,
    remoteImages,
    remoteImagesAllowed,
    onremoteimages,
    themeName,
    onthemeclick,
    typography,
    ontypographyclick,
    tocVisible,
    ontoggletoc,
    propertiesShown = null,
    ontoggleproperties,
    flavor,
    flavorSource,
    onflavor,
    autosave,
    onsettings,
    progress,
    onshortcuts,
    onprint,
  }: Props = $props();

  let flavorMenuOpen = $state(false);
  // "Pandoc" when chosen for this document, "Pandoc (auto)" otherwise.
  const flavorLabel = $derived(`${FLAVOR_NAMES[flavor]}${flavorSource === "document" ? "" : " (auto)"}`);
</script>

<footer class="sk-status" style:--sk-progress={progress}>
  <button
    class="toc"
    onclick={ontoggletoc}
    aria-pressed={tocVisible}
    title={`${tocVisible ? "Hide" : "Show"} contents (Ctrl+\\)`}
    aria-label="Toggle contents">☰</button
  >
  {#if canGoBack || canGoForward}
    <span class="nav">
      <button onclick={onback} disabled={!canGoBack} title="Back (Alt+Left)" aria-label="Back">‹</button>
      <button onclick={onforward} disabled={!canGoForward} title="Forward (Alt+Right)" aria-label="Forward">›</button>
    </span>
  {/if}
  {#if notice}
    <span class="section notice" role="status" title={notice}>{notice}</span>
  {:else}
    <span class="section" title={section ?? ""}>{section ?? ""}</span>
  {/if}
  {#if remoteImages > 0}
    {@const images = `${remoteImages} remote ${remoteImages === 1 ? "image" : "images"}`}
    <span class="remote">
      {#if remoteImagesAllowed}
        {images} loaded
        <button class="load" onclick={() => onremoteimages(false)} title="Stop loading remote images for this document"
          >Block</button
        >
      {:else}
        {images} blocked
        <button class="load" onclick={() => onremoteimages(true)} title="Load remote images for this document">Load</button>
      {/if}
    </span>
  {/if}
  {#if autosave}
    <button
      class="theme on"
      onclick={onsettings}
      title="Autosave is on: documents save themselves 5 seconds after you stop editing. Click to change it in Settings."
      >Autosave</button
    >
  {/if}
  {#if themeName}
    <button class="theme" onclick={onthemeclick} title="Change theme (Ctrl+T)">{themeName}</button>
  {/if}
  <button class="theme" data-popover-toggle="typography" onclick={ontypographyclick} title="Text size and zoom (Ctrl+wheel, Ctrl+=, Ctrl+−, Ctrl+0)"
    >{typography}</button
  >
  <button
    class="theme"
    data-popover-toggle="flavor"
    title="Markdown flavor"
    aria-haspopup="dialog"
    aria-expanded={flavorMenuOpen}
    onclick={() => (flavorMenuOpen = !flavorMenuOpen)}>{flavorLabel}</button
  >
  {#if flavorMenuOpen}
    <FlavorMenu {flavor} {flavorSource} onchoose={onflavor} onclose={() => (flavorMenuOpen = false)} />
  {/if}
  <span>{formatWordCount(wordCount)}</span>
  <span
    class="progress"
    title={`Read: ${formatProgress(progress)}. Whole document: about ${formatReadingTime(wordCount)}.`}
    >{formatProgress(progress)} · {formatTimeLeft(wordCount, progress)}</span
  >
  <button class="theme help" onclick={onprint} title="Print or save as PDF (Ctrl+P)" aria-label="Print">🖶︎</button>
  <button class="theme help" onclick={onsettings} title="Settings (Ctrl+,)" aria-label="Settings">⚙</button>
  <button class="theme help" onclick={onshortcuts} title="Keyboard shortcuts (F1)" aria-label="Keyboard shortcuts">?</button>
  {#if propertiesShown !== null}
    <button
      class="toc"
      onclick={ontoggleproperties}
      aria-pressed={propertiesShown}
      title={`${propertiesShown ? "Hide" : "Show"} properties (Ctrl+Shift+\\)`}
      aria-label="Toggle properties">◨</button
    >
  {/if}
</footer>

<style>
  .sk-status {
    position: relative;
    display: flex;
    gap: 1.25rem;
    align-items: center;
    padding: 0.25rem 0.75rem;
    font-family: var(--sk-font-ui);
    font-size: 0.75rem;
    color: var(--sk-color-muted);
    background: var(--sk-color-ui-background);
    border-top: 1px solid var(--sk-color-border);
    white-space: nowrap;
  }

  button {
    font: inherit;
    color: var(--sk-color-link);
    background: none;
    border: 1px solid var(--sk-color-border);
    border-radius: 3px;
    padding: 0 0.4em;
    cursor: pointer;
  }

  button:disabled {
    color: var(--sk-color-muted);
    opacity: 0.5;
    cursor: default;
  }

  .toc {
    border-color: transparent;
    color: var(--sk-color-muted);
    font-size: 0.95rem;
    line-height: 1;
    padding: 0.1em 0.35em;
  }

  .toc[aria-pressed="true"] {
    color: var(--sk-color-foreground);
  }

  .nav {
    display: flex;
    gap: 0.25rem;
  }

  .nav button {
    font-size: 1rem;
    line-height: 1;
    padding: 0 0.35em 0.1em;
  }

  .theme {
    border-color: transparent;
    color: var(--sk-color-muted);
  }

  .theme:hover {
    border-color: var(--sk-color-border);
    color: var(--sk-color-foreground);
  }

  .theme.on {
    color: var(--sk-color-foreground);
  }

  .load {
    margin-left: 0.3em;
  }

  .section {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .notice {
    color: var(--sk-color-accent);
  }

  /* Reading progress: a thin line along the top edge, as wide as the share read. */
  .sk-status::before {
    content: "";
    position: absolute;
    top: -1px;
    left: 0;
    height: 2px;
    width: calc(var(--sk-progress, 0) * 100%);
    background: var(--sk-color-accent);
    pointer-events: none;
  }

  .progress {
    font-variant-numeric: tabular-nums;
  }
</style>
