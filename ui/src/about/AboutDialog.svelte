<script lang="ts">
  // About Scrald (DESIGN.md §10): the Skald raising his horn, the version,
  // where the name comes from, and a link to the source. Opened from the
  // start screen and the settings dialog. Escape, the × button, or clicking
  // outside closes it.
  import { getVersion } from "@tauri-apps/api/app";
  import { onMount } from "svelte";
  import skaldUrl from "../assets/art/skald-skal.svg";
  import { openExternal } from "../lib/commands";
  import { trapFocus } from "../lib/focus";

  const SOURCE_URL = "https://github.com/torstees/scrald";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();
  let closeButton: HTMLButtonElement | undefined = $state();
  // Read from the app (tauri.conf.json), so it can't drift from the build.
  let version = $state<string | null>(null);

  onMount(() => {
    closeButton?.focus();
    getVersion()
      .then((v) => (version = v))
      .catch(() => (version = null));
  });

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
  <div class="panel" use:trapFocus role="dialog" aria-modal="true" aria-labelledby="sk-about-title">
    <header>
      <button bind:this={closeButton} class="close" aria-label="Close" title="Close (Esc)" onclick={onclose}>×</button>
    </header>
    <div class="body">
      <img class="art" src={skaldUrl} alt="The Skald, Scrald's mascot, raising a drinking horn: Skål!" width="176" height="176" />
      <div class="text">
        <h2 id="sk-about-title">Scrald</h2>
        {#if version}<p class="version">Version {version}</p>{/if}
        <p>A themed Markdown reader for anything from technical notes to sagas.</p>
        <p>
          The name plays on <em>scrawled</em>, <em>skald</em> (the Norse poet), and <em>skål</em> (the Viking
          toast).
        </p>
        <p class="links">
          <button class="link" onclick={() => void openExternal(SOURCE_URL)} title={SOURCE_URL}>Source on GitHub</button>
          <span class="muted">MIT License</span>
        </p>
      </div>
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
    padding: 10vh 1rem 0;
    z-index: 11;
  }

  .panel {
    width: min(36em, 100%);
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
    justify-content: flex-end;
    padding: 0.5em 0.6em 0;
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

  .body {
    display: flex;
    gap: 1.5em;
    align-items: center;
    padding: 0 1.75em 1.75em;
  }

  .art {
    flex: none;
    width: 11em;
    height: 11em;
  }

  .text {
    min-width: 0;
  }

  h2 {
    margin: 0;
    font-family: var(--sk-font-heading);
    font-size: 2em;
    font-weight: 400;
  }

  p {
    margin: 0.6em 0 0;
    line-height: 1.5;
  }

  .version,
  .muted {
    color: var(--sk-color-muted);
  }

  .version {
    margin-top: 0.1em;
  }

  .links {
    display: flex;
    gap: 1.2em;
    align-items: baseline;
  }

  button.link {
    font: inherit;
    border: none;
    background: none;
    padding: 0;
    color: var(--sk-color-link);
    cursor: pointer;
  }

  button.link:hover {
    text-decoration: underline;
  }

  /* Narrow windows: the Skald above the text. */
  @media (max-width: 30em) {
    .body {
      flex-direction: column;
      text-align: center;
    }

    .links {
      justify-content: center;
    }
  }
</style>
