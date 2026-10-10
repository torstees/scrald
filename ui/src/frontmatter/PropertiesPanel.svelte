<script lang="ts">
  // Properties panel (DESIGN.md §8.2): a collapsible view of the front matter
  // at the top of the document. Read-only for now; fields that the
  // minimal-edit engine can't change safely say why (§8.3).
  import type { FrontMatter } from "../lib/types";
  import { isWebLink, propertyRows } from "./properties";

  interface Props {
    frontMatter: FrontMatter;
    open: boolean;
    ontoggle: (open: boolean) => void;
    /** Opens a link the same way links in the document do. */
    onlink: (href: string) => void;
  }

  let { frontMatter, open, ontoggle, onlink }: Props = $props();

  const rows = $derived(propertyRows(frontMatter));
</script>

{#if rows.length > 0 || frontMatter.error}
  <section class="sk-properties" aria-label="Properties">
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
        {#each rows as row (row.key)}
          <dt title={row.label === row.key ? undefined : row.key}>
            {row.label}
            {#if !row.editable && row.reason}
              <span class="sk-readonly" title={`Can't be edited here: ${row.reason}. Edit it in raw mode.`}>read-only</span>
            {/if}
          </dt>
          <dd class:multiline={row.kind === "multiline"}>
            {#if Array.isArray(row.value)}
              {#each row.value as item, i (i)}
                <span class={row.kind === "tags" ? "sk-chip" : "sk-item"}>{row.kind === "tags" ? `#${item}` : item}</span>
              {/each}
            {:else if row.kind === "link" && isWebLink(row.value)}
              <a
                href={row.value}
                onclick={(event) => {
                  event.preventDefault();
                  onlink(row.value as string);
                }}>{row.value}</a
              >
            {:else}
              {row.value}
            {/if}
          </dd>
        {/each}
      </dl>
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
    display: block;
  }

  .sk-chip {
    color: var(--sk-color-accent);
    background: color-mix(in srgb, var(--sk-color-accent) 12%, transparent);
    border-radius: 999px;
    padding: 0.05em 0.6em;
  }

  .sk-item:not(:last-child)::after {
    content: ",";
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
    cursor: help;
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
