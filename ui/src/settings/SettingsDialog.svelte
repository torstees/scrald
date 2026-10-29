<script lang="ts">
  // Global settings (DESIGN.md §10), opened with Ctrl+, or the ⚙ button.
  // Each change applies at once, in every window; there's no Save button.
  // Escape, the × button, or clicking outside closes it.
  import { MOD } from "../lib/platform";
  import { trapFocus } from "../lib/focus";
  import { onMount } from "svelte";
  import type {
    FrontChoice,
    FrontSettings,
    NewDocumentOptions,
    PropertiesPlacement,
    ThemeSummary,
    TypographyDefaults,
  } from "../lib/types";
  import { formatZoom, ZOOM_STEPS, type TextSizing } from "../typography/typography";

  interface Props {
    themes: ThemeSummary[];
    /** The global default theme, or null to follow the system's light or dark mode. */
    defaultTheme: string | null;
    typography: TypographyDefaults;
    contentsDepth: number;
    propertiesOpen: boolean;
    propertiesPlacement: PropertiesPlacement;
    autosave: boolean;
    /** Whether documents show their cover and a missing title (#189). */
    front: FrontSettings;
    ondefaulttheme: (id: string | null) => void;
    ontypography: (defaults: TypographyDefaults) => void;
    oncontentsdepth: (depth: number) => void;
    onpropertiesopen: (open: boolean) => void;
    onpropertiesplacement: (placement: PropertiesPlacement) => void;
    onautosave: (enabled: boolean) => void;
    onfront: (settings: FrontSettings) => void;
    /** Remembered folders and the "Remember this folder" default; null while loading. */
    newDocument: NewDocumentOptions | null;
    onremembernewfolders: (remember: boolean) => void;
    onforgetfolder: (path: string) => void;
    /** Picks a folder to remember. */
    onaddfolder: () => void;
    /** Opens About Scrald (the dialog closes first). */
    onabout: () => void;
    onclose: () => void;
  }

  let {
    themes,
    defaultTheme,
    typography,
    contentsDepth,
    propertiesOpen,
    propertiesPlacement,
    autosave,
    front,
    ondefaulttheme,
    ontypography,
    oncontentsdepth,
    onpropertiesopen,
    onpropertiesplacement,
    onautosave,
    onfront,
    newDocument,
    onremembernewfolders,
    onforgetfolder,
    onaddfolder,
    onabout,
    onclose,
  }: Props = $props();

  const rememberedFolders = $derived(newDocument?.folders.filter((f) => f.remembered) ?? []);

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
  <div class="panel" use:trapFocus role="dialog" aria-modal="true" aria-labelledby="sk-settings-title">
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
        <label class="row">
          <span class="label">Properties</span>
          <select
            value={propertiesPlacement}
            onchange={(e) => onpropertiesplacement(e.currentTarget.value as PropertiesPlacement)}
          >
            <option value="top">At the top of the document</option>
            <option value="left">At the left, as a tab beside Contents</option>
            <option value="right">At the right, in a sidebar of their own</option>
          </select>
        </label>
        <label class="row check">
          <input type="checkbox" checked={propertiesOpen} onchange={(e) => onpropertiesopen(e.currentTarget.checked)} />
          Show the properties open
        </label>
        <p class="hint">A document's front matter. At the left or right, a document without any can have properties added.</p>
        <label class="row">
          <span class="label">Cover image</span>
          <select value={front.cover} onchange={(e) => onfront({ ...front, cover: e.currentTarget.value as FrontChoice })}>
            <option value="theme">As the theme says</option>
            <option value="show">Always show</option>
            <option value="hide">Never show</option>
          </select>
        </label>
        <label class="row">
          <span class="label">Missing title</span>
          <select value={front.title} onchange={(e) => onfront({ ...front, title: e.currentTarget.value as FrontChoice })}>
            <option value="theme">As the theme says</option>
            <option value="show">Always show</option>
            <option value="hide">Never show</option>
          </select>
        </label>
        <p class="hint">
          A cover named in the front matter shows above the text. Without one, a front matter title the text doesn't
          start with shows as a heading. Neither changes the file.
        </p>
      </section>

      <section>
        <h3>Editing</h3>
        <label class="row check">
          <input type="checkbox" checked={autosave} onchange={(e) => onautosave(e.currentTarget.checked)} />
          Autosave
        </label>
        <p class="hint">Saves a document 5 seconds after your last edit. When off, <kbd>{MOD}</kbd>+<kbd>S</kbd> saves, and closing asks first.</p>
      </section>

      <section>
        <h3>New documents</h3>
        {#if newDocument}
          <label class="row check">
            <input
              type="checkbox"
              checked={newDocument.rememberByDefault}
              onchange={(e) => onremembernewfolders(e.currentTarget.checked)}
            />
            Check "Remember this folder" to start with
          </label>
          <p class="hint">Remembered folders are offered first when you create a document (<kbd>{MOD}</kbd>+<kbd>N</kbd>), each with the template you last used there.</p>
          {#if rememberedFolders.length > 0}
            <ul class="folders">
              {#each rememberedFolders as folder (folder.path)}
                <li class:missing={!folder.exists}>
                  <span class="path" title={folder.path}>{folder.path}</span>
                  {#if !folder.exists}<span class="missing-note">Not found</span>{/if}
                  <button
                    class="remove"
                    aria-label={`Forget ${folder.path}`}
                    title="Forget this folder"
                    onclick={() => onforgetfolder(folder.path)}>×</button
                  >
                </li>
              {/each}
            </ul>
          {/if}
          <button class="add" onclick={onaddfolder}>Add folder…</button>
        {/if}
      </section>
    </div>
    <footer>
      <button class="about" onclick={onabout}>About Scrald</button>
    </footer>
  </div>
</div>

<style>
  footer {
    padding: 0.5em 1em 0.7em;
    border-top: 1px solid var(--sk-color-border);
  }

  button.about {
    font: inherit;
    border: none;
    background: none;
    padding: 0;
    color: var(--sk-color-link);
    cursor: pointer;
  }

  button.about:hover {
    text-decoration: underline;
  }

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

  .folders {
    list-style: none;
    margin: 0 0 0.5em;
    padding: 0;
  }

  .folders li {
    display: flex;
    align-items: center;
    gap: 0.5em;
    padding: 0.15em 0;
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .missing .path {
    color: var(--sk-color-muted);
  }

  .missing-note {
    color: var(--sk-color-muted);
    font-size: 0.9em;
  }

  .remove,
  .add {
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    cursor: pointer;
  }

  .remove {
    padding: 0 0.45em;
    color: var(--sk-color-muted);
    background: none;
    border-color: transparent;
  }

  .remove:hover {
    color: var(--sk-color-foreground);
    border-color: var(--sk-color-border);
  }

  .add {
    padding: 0.25em 0.7em;
  }

  code,
  kbd {
    font-family: var(--sk-font-mono);
    font-size: 0.95em;
  }
</style>
