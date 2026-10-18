<script lang="ts">
  // Global settings (DESIGN.md §10), opened with Ctrl+, or the ⚙ button.
  // Each change applies at once, in every window; there's no Save button.
  // Escape, the × button, or clicking outside closes it.
  import { onMount } from "svelte";
  import type { ThemeSummary, TypographyDefaults } from "../lib/types";
  import { formatZoom, ZOOM_STEPS, type TextSizing } from "../typography/typography";

  interface Props {
    themes: ThemeSummary[];
    /** The global default theme, or null to follow the system's light or dark mode. */
    defaultTheme: string | null;
    typography: TypographyDefaults;
    contentsDepth: number;
    propertiesOpen: boolean;
    autosave: boolean;
    ondefaulttheme: (id: string | null) => void;
    ontypography: (defaults: TypographyDefaults) => void;
    oncontentsdepth: (depth: number) => void;
    onpropertiesopen: (open: boolean) => void;
    onautosave: (enabled: boolean) => void;
    onclose: () => void;
  }

  let {
    themes,
    defaultTheme,
    typography,
    contentsDepth,
    propertiesOpen,
    autosave,
    ondefaulttheme,
    ontypography,
    oncontentsdepth,
    onpropertiesopen,
    onautosave,
    onclose,
  }: Props = $props();

  let closeButton: HTMLButtonElement | undefined = $state();
  onMount(() => closeButton?.focus());

  const usable = $derived(themes.filter((t) => t.error === null));
  const bundled = $derived(usable.filter((t) => t.bundled));
  const own = $derived(usable.filter((t) => !t.bundled));
  // An id that no longer names a theme still shows, so the list doesn't lie.
  const missingDefault = $derived(defaultTheme !== null && !usable.some((t) => t.id === defaultTheme));

  /** Select values: "" stands for null. */
  function sizingValue(mode: TextSizing | null): string {
    return mode ?? "";
  }

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
  <div class="panel" role="dialog" aria-modal="true" aria-labelledby="sk-settings-title">
    <header>
      <h2 id="sk-settings-title">Settings</h2>
      <button bind:this={closeButton} class="close" aria-label="Close" title="Close (Esc)" onclick={onclose}>×</button>
    </header>
    <div class="body">
      <section>
        <h3>Appearance</h3>
        <label class="row">
          <span class="label">Default theme</span>
          <select
            value={defaultTheme ?? ""}
            onchange={(e) => ondefaulttheme(e.currentTarget.value === "" ? null : e.currentTarget.value)}
          >
            <option value="">Follow the system (Scrald Light or Dark)</option>
            {#if missingDefault}
              <option value={defaultTheme}>{defaultTheme} (not found)</option>
            {/if}
            <optgroup label="Built in">
              {#each bundled as theme (theme.id)}
                <option value={theme.id}>{theme.name}</option>
              {/each}
            </optgroup>
            {#if own.length > 0}
              <optgroup label="Your themes">
                {#each own as theme (theme.id)}
                  <option value={theme.id}>{theme.name}</option>
                {/each}
              </optgroup>
            {/if}
          </select>
        </label>
        <p class="hint">For documents without a theme of their own, in their front matter, or in a folder's <code>.scrald.toml</code>.</p>
      </section>

      <section>
        <h3>Reading</h3>
        <label class="row">
          <span class="label">Text size</span>
          <select
            value={sizingValue(typography.textSizing)}
            onchange={(e) => {
              const value = e.currentTarget.value;
              ontypography({ ...typography, textSizing: value === "" ? null : (value as TextSizing) });
            }}
          >
            <option value="">As the theme suggests</option>
            <option value="fixed">Fixed</option>
            <option value="fit">Fit to window</option>
          </select>
        </label>
        <label class="row">
          <span class="label">Zoom</span>
          <select
            value={typography.zoom ?? 1}
            onchange={(e) => {
              const zoom = Number(e.currentTarget.value);
              ontypography({ ...typography, zoom: zoom === 1 ? null : zoom });
            }}
          >
            {#each ZOOM_STEPS as step (step)}
              <option value={step}>{formatZoom(step)}</option>
            {/each}
          </select>
        </label>
        <p class="hint">For documents you haven't sized or zoomed yourself. "Make this the default" in the status bar's text size menu sets these too.</p>
        <label class="row check">
          <input
            type="checkbox"
            checked={typography.fillWindow}
            onchange={(e) => ontypography({ ...typography, fillWindow: e.currentTarget.checked })}
          />
          Fill the window (ignore the theme's line length)
        </label>
        <label class="row">
          <span class="label">Contents depth</span>
          <select value={contentsDepth} onchange={(e) => oncontentsdepth(Number(e.currentTarget.value))}>
            {#each [1, 2, 3, 4, 5] as level (level)}
              <option value={level}>Down to H{level}</option>
            {/each}
            <option value={6}>All headings</option>
          </select>
        </label>
        <label class="row check">
          <input type="checkbox" checked={propertiesOpen} onchange={(e) => onpropertiesopen(e.currentTarget.checked)} />
          Show the properties panel open
        </label>
      </section>

      <section>
        <h3>Editing</h3>
        <label class="row check">
          <input type="checkbox" checked={autosave} onchange={(e) => onautosave(e.currentTarget.checked)} />
          Autosave
        </label>
        <p class="hint">Saves a document 5 seconds after your last edit. When off, <kbd>Ctrl</kbd>+<kbd>S</kbd> saves, and closing asks first.</p>
      </section>
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
    padding: 6vh 1rem 0;
    z-index: 11;
  }

  .panel {
    width: min(34em, 100%);
    max-height: 86vh;
    display: flex;
    flex-direction: column;
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
    align-items: center;
    padding: 0.8em 1em 0.5em;
    border-bottom: 1px solid var(--sk-color-border);
  }

  h2 {
    flex: 1;
    margin: 0;
    font-size: 1.15em;
    font-family: var(--sk-font-ui);
  }

  .close {
    font: inherit;
    font-size: 1.3em;
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
    overflow-y: auto;
    padding: 0.2em 1em 1em;
  }

  section {
    padding-top: 0.8em;
  }

  h3 {
    margin: 0 0 0.5em;
    font-size: 0.9em;
    font-family: var(--sk-font-ui);
    color: var(--sk-color-accent);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  /* In em of the panel's UI text: rem is the theme's reading size. */
  .row {
    display: flex;
    align-items: center;
    gap: 0.8em;
    margin: 0.45em 0;
  }

  .label {
    flex: 0 0 9em;
  }

  .check {
    gap: 0.5em;
  }

  select {
    flex: 1;
    min-width: 0;
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.25em 0.4em;
  }

  input[type="checkbox"] {
    accent-color: var(--sk-color-accent);
    margin: 0;
  }

  .hint {
    margin: 0.2em 0 0.6em;
    color: var(--sk-color-muted);
    font-size: 0.9em;
    line-height: 1.4;
  }

  code,
  kbd {
    font-family: var(--sk-font-mono);
    font-size: 0.95em;
  }
</style>
