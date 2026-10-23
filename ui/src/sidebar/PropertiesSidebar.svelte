<script lang="ts">
  // The properties panel in a sidebar (DESIGN.md §8.2): as a tab beside
  // Contents at the left, or on its own at the right. It uses the UI's text
  // size, like Contents, not the reading column's size and zoom.
  import { keyLabel } from "../lib/platform";
  import type { Snippet } from "svelte";
  import PropertiesPanel from "../frontmatter/PropertiesPanel.svelte";
  import PlacementSelect from "../frontmatter/PlacementSelect.svelte";
  import type { PropertyEditor } from "../frontmatter/properties";
  import type { FrontMatter, PropertiesPlacement } from "../lib/types";

  interface Props {
    side: "left" | "right";
    frontMatter: FrontMatter | null;
    editor: PropertyEditor | null;
    onlink: (href: string) => void;
    onraw: (offset: number) => void;
    onplacement: (placement: PropertiesPlacement) => void;
    /** Hides the sidebar (the right one; the left one has Contents' toggle). */
    onclose?: () => void;
    /** Tabs shown instead of the title, when Contents shares the sidebar. */
    tabs?: Snippet;
  }

  let { side, frontMatter, editor, onlink, onraw, onplacement, onclose, tabs }: Props = $props();
</script>

<aside class="sk-props-sidebar {side}" aria-label="Properties">
  <header>
    {#if tabs}
      {@render tabs()}
    {:else}
      <span class="title">Properties</span>
    {/if}
    <span class="actions">
      <PlacementSelect placement={side} {onplacement} />
      {#if onclose}
        <button class="close" aria-label="Hide properties" title={keyLabel("Hide properties (Ctrl+Shift+\\)")} onclick={onclose}>×</button>
      {/if}
    </span>
  </header>
  <PropertiesPanel {frontMatter} open={true} ontoggle={() => {}} docked {editor} {onlink} {onraw} />
</aside>

<style>
  /* Matches the Contents sidebar. */
  .sk-props-sidebar {
    height: 100%;
    min-width: 0;
    overflow-y: auto;
    box-sizing: border-box;
    padding: 0.75rem 0.5rem 2rem;
    font-family: var(--sk-font-ui);
    font-size: 0.8rem;
    color: var(--sk-color-foreground);
    background: var(--sk-color-ui-background);
  }

  .left {
    border-right: 1px solid var(--sk-color-border);
  }

  .right {
    border-left: 1px solid var(--sk-color-border);
  }

  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0 0.25rem 0.5rem;
    color: var(--sk-color-muted);
  }

  .title {
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.7rem;
  }

  .actions {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }

  .close {
    font: inherit;
    font-size: 1.1rem;
    line-height: 1;
    color: var(--sk-color-muted);
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    padding: 0 0.3rem;
    cursor: pointer;
  }

  .close:hover,
  .close:focus-visible {
    color: var(--sk-color-foreground);
    border-color: var(--sk-color-border);
  }
</style>
