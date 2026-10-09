<script lang="ts">
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
    onallowremote: () => void;
    /** Name of the document's theme; clicking it opens the switcher. */
    themeName: string | null;
    onthemeclick: () => void;
    /** Text sizing and zoom summary; clicking it opens the typography panel. */
    typography: string;
    ontypographyclick: () => void;
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
    onallowremote,
    themeName,
    onthemeclick,
    typography,
    ontypographyclick,
  }: Props = $props();
</script>

<footer class="sk-status">
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
  {#if remoteImages > 0 && !remoteImagesAllowed}
    <span class="remote">
      {remoteImages} remote {remoteImages === 1 ? "image" : "images"} blocked
      <button class="load" onclick={onallowremote} title="Load remote images for this document">Load</button>
    </span>
  {/if}
  {#if themeName}
    <button class="theme" onclick={onthemeclick} title="Change theme (Ctrl+T)">{themeName}</button>
  {/if}
  <button class="theme" data-typography-toggle onclick={ontypographyclick} title="Text size and zoom (Ctrl+wheel, Ctrl+=, Ctrl+−, Ctrl+0)"
    >{typography}</button
  >
  <span>{formatWordCount(wordCount)}</span>
  <span title="Estimated reading time">{formatReadingTime(wordCount)}</span>
</footer>

<style>
  .sk-status {
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
</style>
