<script lang="ts">
  // New document (DESIGN.md §11), opened with Ctrl+N or from the start
  // screen: a title (optional), a template, a file name, and a folder.
  // Enter creates, Escape cancels. The app writes the file; the dialog stays
  // open with the reason when it can't.
  import { trapFocus } from "../lib/focus";
  import { onMount } from "svelte";
  import type { FolderChoice, NewDocumentOptions, NewDocumentRequest } from "../lib/types";
  import { asSentence, fileNameForTitle, folderName, localDate, PLAIN, templateFor, UNTITLED } from "./newdoc";

  interface Props {
    options: NewDocumentOptions;
    /** Creates the document; rejects with a message to show. */
    oncreate: (request: NewDocumentRequest) => Promise<void>;
    /** The system folder dialog; resolves to null when cancelled. */
    onpickfolder: (near: string | null) => Promise<string | null>;
    ontemplatesfolder: () => void;
    oncancel: () => void;
  }

  let { options, oncreate, onpickfolder, ontemplatesfolder, oncancel }: Props = $props();

  // The dialog edits its own copies; `options` is only where it starts.
  // svelte-ignore state_referenced_locally
  let folders = $state<FolderChoice[]>(options.folders.filter((f) => f.exists));
  // svelte-ignore state_referenced_locally
  let folder = $state(options.defaultFolder ?? folders[0]?.path ?? "");
  // svelte-ignore state_referenced_locally
  let template = $state(templateFor(folders.find((f) => f.path === folder), options.templates));
  // svelte-ignore state_referenced_locally
  let remember = $state(options.rememberByDefault);
  let title = $state("");
  let fileName = $state(UNTITLED);
  /** Once the name is typed by hand, the title stops changing it. */
  let nameEdited = false;
  let problem = $state<string | null>(null);
  let creating = $state(false);

  const hasTitle = $derived(title.trim() !== "");
  const chosen = $derived(folders.find((f) => f.path === folder));
  const CHOOSE = "\u0000choose";

  let titleInput: HTMLInputElement | undefined = $state();
  onMount(() => titleInput?.focus());

  function onTitle(text: string): void {
    title = text;
    if (!nameEdited) fileName = fileNameForTitle(text);
  }

  async function onFolder(select: HTMLSelectElement): Promise<void> {
    if (select.value !== CHOOSE) {
      useFolder(select.value);
      return;
    }
    // Back to the current folder while the system dialog is up.
    select.value = folder;
    const picked = await onpickfolder(folder || null);
    if (!picked) return;
    if (!folders.some((f) => f.path === picked)) {
      folders = [...folders, { path: picked, remembered: false, template: null, exists: true }];
    }
    useFolder(picked);
  }

  function useFolder(path: string): void {
    folder = path;
    template = templateFor(folders.find((f) => f.path === path), options.templates);
  }

  async function create(): Promise<void> {
    if (creating) return;
    if (!folder) {
      problem = "Choose a folder for the document.";
      return;
    }
    creating = true;
    problem = null;
    try {
      await oncreate({
        folder,
        fileName,
        title: hasTitle ? title.trim() : null,
        template: hasTitle ? template : PLAIN,
        date: localDate(new Date()),
        remember: remember && !chosen?.remembered,
      });
    } catch (e) {
      problem = asSentence(String(e));
    } finally {
      creating = false;
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      oncancel();
    } else if (event.key === "Enter" && (event.target as HTMLElement).tagName === "INPUT") {
      // Enter in a text field creates; on a button or a menu it does what
      // that control does.
      event.preventDefault();
      void create();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onkeydown={onKeydown} onclick={(e) => e.target === e.currentTarget && oncancel()}>
  <div class="panel" use:trapFocus role="dialog" aria-modal="true" aria-labelledby="sk-newdoc-title">
    <header>
      <h2 id="sk-newdoc-title">New document</h2>
    </header>
    <div class="body">
      <label class="row">
        <span class="label">Title</span>
        <input
          bind:this={titleInput}
          type="text"
          placeholder="Optional"
          value={title}
          oninput={(e) => onTitle(e.currentTarget.value)}
        />
      </label>
      <p class="hint">Written to the front matter as <code>title</code>. Leave it empty for a file without front matter, such as a README.</p>

      <label class="row">
        <span class="label">Template</span>
        <select value={hasTitle ? template : PLAIN} disabled={!hasTitle} onchange={(e) => (template = e.currentTarget.value)}>
          {#each options.templates as t (t.id)}
            <option value={t.id}>{t.name}</option>
          {/each}
        </select>
      </label>
      <p class="hint">
        {#if hasTitle}
          A template sets the front matter and, for the built-in ones, the flavor.
        {:else}
          Add a title to use a template.
        {/if}
        {#if options.templatesFolder}
          <button class="link" onclick={ontemplatesfolder} title={options.templatesFolder}>Templates folder</button>
        {/if}
      </p>

      <label class="row">
        <span class="label">File name</span>
        <input
          type="text"
          spellcheck="false"
          value={fileName}
          oninput={(e) => {
            fileName = e.currentTarget.value;
            nameEdited = true;
          }}
        />
      </label>

      <label class="row">
        <span class="label">Folder</span>
        <select value={folder} onchange={(e) => void onFolder(e.currentTarget)} title={folder}>
          {#each folders as f (f.path)}
            <option value={f.path}>{folderName(f.path)}{f.remembered ? " ★" : ""} · {f.path}</option>
          {/each}
          <option value={CHOOSE}>Choose folder…</option>
        </select>
      </label>
      {#if chosen && !chosen.remembered}
        <label class="row check">
          <input type="checkbox" checked={remember} onchange={(e) => (remember = e.currentTarget.checked)} />
          Remember this folder
        </label>
      {/if}
      <p class="hint">★ marks remembered folders. Manage them in Settings.</p>

      {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
    </div>
    <footer>
      <button class="primary" disabled={creating} onclick={() => void create()}>Create</button>
      <button onclick={oncancel}>Cancel</button>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 8vh 1rem 0;
    z-index: 11;
  }

  .panel {
    width: min(32em, 100%);
    max-height: 84vh;
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
    padding: 0.8em 1em 0.5em;
    border-bottom: 1px solid var(--sk-color-border);
  }

  h2 {
    margin: 0;
    font-size: 1.15em;
    font-family: var(--sk-font-ui);
  }

  .body {
    overflow-y: auto;
    padding: 0.6em 1em 0.4em;
  }

  /* In em of the panel's UI text: rem is the theme's reading size. */
  .row {
    display: flex;
    align-items: center;
    gap: 0.8em;
    margin: 0.45em 0;
  }

  .label {
    flex: 0 0 6em;
  }

  .check {
    gap: 0.5em;
    margin-left: 6.8em;
  }

  input[type="text"],
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

  select:disabled {
    opacity: 0.6;
  }

  input[type="checkbox"] {
    accent-color: var(--sk-color-accent);
    margin: 0;
  }

  .hint {
    margin: 0.1em 0 0.6em 6.8em;
    color: var(--sk-color-muted);
    font-size: 0.9em;
    line-height: 1.4;
  }

  .problem {
    margin: 0.4em 0;
    color: var(--sk-palette-red, var(--sk-color-accent));
    overflow-wrap: anywhere;
  }

  code {
    font-family: var(--sk-font-mono);
    font-size: 0.95em;
  }

  .link {
    font: inherit;
    color: var(--sk-color-link);
    background: none;
    border: none;
    padding: 0;
    margin-left: 0.3em;
    text-decoration: underline;
    cursor: pointer;
  }

  footer {
    display: flex;
    gap: 0.5em;
    justify-content: flex-end;
    padding: 0.6em 1em 0.8em;
    border-top: 1px solid var(--sk-color-border);
  }

  footer button {
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.3em 0.9em;
    cursor: pointer;
  }

  footer .primary {
    border-color: var(--sk-color-accent);
    font-weight: 600;
  }

  footer button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
