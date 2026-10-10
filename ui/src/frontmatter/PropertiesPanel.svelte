<script lang="ts">
  // Properties panel (DESIGN.md §8.2): the front matter at the top of the
  // document. With an editor, values are edited in place; each change is a
  // minimal text edit in memory (§8.3), saved with Ctrl+S like any edit.
  // Keys the engine can't change safely are read-only and say why.
  import { tick } from "svelte";
  import type { Flavor, FrontMatter, PropertyChange } from "../lib/types";
  import { FLAVOR_NAMES } from "../status/flavor";
  import {
    isWebLink,
    missingProperties,
    propertyRows,
    type NewProperty,
    type PropertyEditor,
    type PropertyRow,
  } from "./properties";

  interface Props {
    frontMatter: FrontMatter;
    open: boolean;
    ontoggle: (open: boolean) => void;
    /** Opens a link the same way links in the document do. */
    onlink: (href: string) => void;
    editor?: PropertyEditor | null;
  }

  let { frontMatter, open, ontoggle, onlink, editor = null }: Props = $props();

  const rows = $derived(propertyRows(frontMatter));
  const missing = $derived(editor ? missingProperties(frontMatter) : []);

  // The row being edited (its key), and the text being typed.
  let editing = $state<string | null>(null);
  let draft = $state("");
  // A property being added: shown as an extra row in edit mode.
  let adding = $state<NewProperty | null>(null);
  let addMenuOpen = $state(false);
  // Text typed into a list's "add" box, per key.
  let newItem = $state<Record<string, string>>({});
  let busy = $state(false);

  const FLAVORS: Flavor[] = ["gfm", "obsidian", "pandoc"];

  async function apply(key: string, change: PropertyChange): Promise<void> {
    if (!editor || busy) return;
    busy = true;
    try {
      await editor.edit(key, change);
    } finally {
      busy = false;
    }
  }

  async function startEdit(key: string, value: string): Promise<void> {
    editing = key;
    draft = value;
    await tick();
    const field = document.querySelector<HTMLInputElement | HTMLTextAreaElement>(".sk-properties [data-editing]");
    field?.focus();
    field?.select();
  }

  function cancelEdit(): void {
    editing = null;
    adding = null;
  }

  /** Commits the draft: text, a typed value, or removal when emptied. */
  async function commitText(key: string, original: string, typed: boolean): Promise<void> {
    if (editing !== key) return;
    const value = draft;
    editing = null;
    adding = null;
    if (value === original) return;
    if (value.trim() === "") {
      if (original !== "") await apply(key, { kind: "remove" });
      return;
    }
    await apply(key, typed ? { kind: "value", value } : { kind: "text", value });
  }

  function onTextKey(event: KeyboardEvent, key: string, original: string, typed: boolean, multiline: boolean): void {
    if (event.key === "Escape") {
      event.preventDefault();
      cancelEdit();
    } else if (event.key === "Enter" && (!multiline || event.ctrlKey)) {
      event.preventDefault();
      void commitText(key, original, typed);
    }
  }

  function listItems(row: PropertyRow | null): string[] {
    return row && Array.isArray(row.value) ? row.value : [];
  }

  async function removeItem(key: string, items: string[], index: number): Promise<void> {
    await apply(key, { kind: "list", value: items.filter((_, i) => i !== index) });
  }

  /** Adds what's in the list's add box (comma-separated), skipping repeats. */
  async function addItems(key: string, items: string[], tags: boolean): Promise<void> {
    const typed = (newItem[key] ?? "")
      .split(",")
      .map((s) => (tags ? s.trim().replace(/^#+/, "") : s.trim()))
      .filter((s) => s !== "" && !items.includes(s));
    newItem[key] = "";
    if (typed.length === 0) return;
    adding = null;
    await apply(key, { kind: "list", value: [...items, ...typed] });
  }

  function onItemKey(event: KeyboardEvent, key: string, items: string[], tags: boolean): void {
    if (event.key === "Enter" || event.key === ",") {
      event.preventDefault();
      void addItems(key, items, tags);
    } else if (event.key === "Escape") {
      newItem[key] = "";
      adding = null;
      (event.target as HTMLElement).blur();
    }
  }

  async function choose(key: string, value: string | null): Promise<void> {
    editing = null;
    adding = null;
    await apply(key, value === null ? { kind: "remove" } : { kind: "text", value });
  }

  async function pickFolder(key: string): Promise<void> {
    if (!editor) return;
    const folder = await editor.pickFolder();
    if (folder === null) return;
    editing = null;
    adding = null;
    await apply(key, { kind: "text", value: folder });
  }

  async function startAdding(property: NewProperty): Promise<void> {
    addMenuOpen = false;
    adding = property;
    if (property.kind === "list" || property.kind === "tags") {
      await tick();
      document.querySelector<HTMLInputElement>(`.sk-properties [data-add-item="${property.key}"]`)?.focus();
    } else {
      await startEdit(property.key, "");
    }
  }

  function themeName(id: string): string {
    return editor?.themes.find((t) => t.id === id)?.name ?? id;
  }

  /** The rows to show: the document's, plus the one being added. */
  const shown = $derived<PropertyRow[]>(
    adding
      ? [
          ...rows,
          {
            label: adding.label,
            key: adding.key,
            kind: adding.kind,
            value: adding.kind === "list" || adding.kind === "tags" ? [] : "",
            editable: true,
            reason: null,
            property: adding.property,
            typed: false,
          },
        ]
      : rows,
  );
</script>

{#snippet chooser(row: PropertyRow)}
  {@const isTheme = row.property === "theme"}
  <div class="sk-options" role="listbox" aria-label={row.label}>
    {#each isTheme ? (editor?.themes ?? []).filter((t) => t.error === null).map((t) => t.id) : FLAVORS as option (option)}
      <button
        role="option"
        aria-selected={row.value === option}
        class:selected={row.value === option}
        onclick={() => choose(row.key, option)}
      >
        {isTheme ? themeName(option) : FLAVOR_NAMES[option as Flavor]}
      </button>
    {/each}
    {#if row.value !== ""}
      <button role="option" aria-selected="false" onclick={() => choose(row.key, null)}>None (remove)</button>
    {/if}
    <button class="sk-cancel" onclick={cancelEdit}>Cancel</button>
  </div>
{/snippet}

{#if shown.length > 0 || frontMatter.error}
  <section class="sk-properties" aria-label="Properties" aria-busy={busy}>
    <button class="sk-properties-toggle" aria-expanded={open} onclick={() => ontoggle(!open)}>
      <span class="sk-chevron" class:open aria-hidden="true">›</span>
      Properties
      {#if !open}<span class="sk-count">{rows.length}</span>{/if}
    </button>
    {#if open}
      {#if frontMatter.error}
        <p class="sk-properties-error">The front matter has a YAML error, so its properties can't be shown: {frontMatter.error}</p>
      {/if}
      <dl>
        {#each shown as row (row.key)}
          {@const canEdit = editor !== null && row.editable}
          {@const items = listItems(row)}
          <dt title={row.label === row.key ? undefined : row.key}>
            {row.label}
            {#if !row.editable && row.reason}
              <span class="sk-readonly" title={`Can't be edited here: ${row.reason}. Edit it in raw mode.`}>read-only</span>
            {/if}
          </dt>
          <dd class:multiline={row.kind === "multiline"}>
            {#if Array.isArray(row.value)}
              {#each items as item, i (i)}
                <span class={row.kind === "tags" ? "sk-chip" : "sk-item"}>
                  {row.kind === "tags" ? `#${item}` : item}
                  {#if canEdit}
                    <button class="sk-x" aria-label={`Remove ${item}`} onclick={() => removeItem(row.key, items, i)}>×</button>
                  {/if}
                </span>
              {/each}
              {#if canEdit}
                <input
                  class="sk-add-item"
                  data-add-item={row.key}
                  placeholder={row.kind === "tags" ? "Add tag" : "Add"}
                  aria-label={`Add to ${row.label}`}
                  bind:value={newItem[row.key]}
                  onkeydown={(e) => onItemKey(e, row.key, items, row.kind === "tags")}
                  onblur={() => addItems(row.key, items, row.kind === "tags")}
                />
              {/if}
            {:else if canEdit && editing === row.key && (row.property === "theme" || row.property === "flavor")}
              {@render chooser(row)}
            {:else if canEdit && editing === row.key && row.kind === "multiline"}
              <textarea
                data-editing
                rows="3"
                bind:value={draft}
                aria-label={row.label}
                onkeydown={(e) => onTextKey(e, row.key, row.value as string, row.typed, true)}
                onblur={() => commitText(row.key, row.value as string, row.typed)}
              ></textarea>
            {:else if canEdit && editing === row.key}
              <input
                data-editing
                bind:value={draft}
                aria-label={row.label}
                onkeydown={(e) => onTextKey(e, row.key, row.value as string, row.typed, false)}
                onblur={() => commitText(row.key, row.value as string, row.typed)}
              />
              {#if row.property === "assets"}
                <!-- onmousedown: pick before the input's blur commits the draft. -->
                <button class="sk-small" onmousedown={(e) => e.preventDefault()} onclick={() => pickFolder(row.key)}>Choose folder…</button>
              {/if}
            {:else if row.kind === "link" && isWebLink(row.value)}
              <a
                href={row.value}
                onclick={(event) => {
                  event.preventDefault();
                  onlink(row.value as string);
                }}>{row.value}</a
              >
              {#if canEdit}
                <button class="sk-small" aria-label={`Edit ${row.label}`} onclick={() => startEdit(row.key, row.value as string)}>Edit</button>
              {/if}
            {:else if canEdit}
              <button
                class="sk-value"
                title="Click to edit"
                onclick={() =>
                  row.property === "theme" || row.property === "flavor"
                    ? (editing = row.key)
                    : startEdit(row.key, row.value as string)}
              >
                {#if row.property === "theme" && row.value !== ""}{themeName(row.value as string)}
                {:else if row.property === "flavor" && row.value !== ""}{FLAVOR_NAMES[row.value as Flavor] ?? row.value}
                {:else if row.value === ""}<span class="sk-empty">Empty</span>
                {:else}{row.value}{/if}
              </button>
            {:else}
              {row.value}
            {/if}
            {#if canEdit && adding?.key !== row.key}
              <button class="sk-x sk-remove" aria-label={`Remove ${row.label}`} title="Remove this property" onclick={() => apply(row.key, { kind: "remove" })}>×</button>
            {/if}
          </dd>
        {/each}
      </dl>
      {#if missing.length > 0 && !adding}
        <div class="sk-add">
          <button class="sk-small" aria-expanded={addMenuOpen} onclick={() => (addMenuOpen = !addMenuOpen)}>+ Add property</button>
          {#if addMenuOpen}
            <div class="sk-options" role="menu">
              {#each missing as property (property.key)}
                <button role="menuitem" onclick={() => startAdding(property)}>{property.label}</button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .sk-properties {
    font-family: var(--sk-font-ui);
    font-size: 0.8em;
    margin: 0 0 2em;
    border: 1px solid var(--sk-color-border);
    border-radius: 6px;
    color: var(--sk-color-foreground);
  }

  .sk-properties[aria-busy="true"] {
    cursor: progress;
  }

  .sk-properties-toggle {
    display: flex;
    align-items: center;
    gap: 0.4em;
    width: 100%;
    padding: 0.5em 0.8em;
    background: none;
    border: none;
    color: var(--sk-color-muted);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .sk-chevron {
    display: inline-block;
    transition: transform 120ms ease;
  }

  .sk-chevron.open {
    transform: rotate(90deg);
  }

  .sk-count {
    margin-left: auto;
    font-size: 0.9em;
  }

  dl {
    display: grid;
    grid-template-columns: minmax(6em, max-content) 1fr;
    gap: 0.45em 1.2em;
    margin: 0;
    padding: 0.2em 0.8em 0.8em;
  }

  dt {
    color: var(--sk-color-muted);
    overflow-wrap: anywhere;
    padding-top: 0.15em;
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
    display: flex;
    flex-wrap: wrap;
    gap: 0.3em 0.4em;
    align-items: baseline;
  }

  dd.multiline {
    white-space: pre-wrap;
  }

  .sk-chip {
    color: var(--sk-color-accent);
    background: color-mix(in srgb, var(--sk-color-accent) 12%, transparent);
    border-radius: 999px;
    padding: 0.05em 0.6em;
  }

  .sk-item:not(:last-of-type)::after {
    content: ",";
  }

  button {
    font: inherit;
    color: inherit;
  }

  .sk-value {
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 0.05em 0.3em;
    margin: -0.1em -0.3em;
    text-align: left;
    white-space: inherit;
    cursor: text;
  }

  .sk-value:hover,
  .sk-value:focus-visible {
    border-color: var(--sk-color-border);
  }

  .sk-empty {
    color: var(--sk-color-muted);
    font-style: italic;
  }

  .sk-x {
    background: none;
    border: none;
    padding: 0 0.15em;
    color: var(--sk-color-muted);
    cursor: pointer;
  }

  .sk-x:hover {
    color: var(--sk-color-foreground);
  }

  /* The row's remove button shows on hover or focus. */
  .sk-remove {
    margin-left: auto;
    opacity: 0;
  }

  dd:hover .sk-remove,
  .sk-remove:focus-visible {
    opacity: 1;
  }

  input,
  textarea {
    font: inherit;
    color: var(--sk-color-foreground);
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.15em 0.4em;
    flex: 1;
    min-width: 8em;
  }

  textarea {
    width: 100%;
    resize: vertical;
  }

  .sk-add-item {
    flex: 0 1 8em;
    border-style: dashed;
    background: none;
  }

  .sk-small {
    background: none;
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.05em 0.5em;
    color: var(--sk-color-muted);
    cursor: pointer;
  }

  .sk-small:hover {
    color: var(--sk-color-foreground);
  }

  .sk-options {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3em;
  }

  .sk-add .sk-options {
    margin-top: 0.4em;
  }

  .sk-options button {
    background: var(--sk-color-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 4px;
    padding: 0.1em 0.6em;
    cursor: pointer;
  }

  .sk-options button:hover,
  .sk-options button.selected {
    border-color: var(--sk-color-accent);
    color: var(--sk-color-accent);
  }

  .sk-options .sk-cancel {
    color: var(--sk-color-muted);
  }

  .sk-add {
    padding: 0 0.8em 0.8em;
  }

  .sk-readonly {
    display: block;
    font-size: 0.85em;
    opacity: 0.8;
    cursor: help;
  }

  .sk-properties-error {
    margin: 0 0.8em 0.6em;
    color: var(--sk-color-muted);
  }

  a {
    color: var(--sk-color-link);
  }

  @media (max-width: 30rem) {
    dl {
      grid-template-columns: 1fr;
      gap: 0.15em;
    }

    dd {
      margin-bottom: 0.5em;
    }
  }
</style>
