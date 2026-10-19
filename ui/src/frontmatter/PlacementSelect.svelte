<script lang="ts">
  // Where the properties panel goes (DESIGN.md §8.2): one global setting,
  // changed here or in the settings dialog.
  import type { PropertiesPlacement } from "../lib/types";

  interface Props {
    placement: PropertiesPlacement;
    onplacement: (placement: PropertiesPlacement) => void;
  }

  let { placement, onplacement }: Props = $props();
</script>

<label class="sk-placement" title="Where the properties panel goes, in every window">
  Show
  <select value={placement} onchange={(e) => onplacement(e.currentTarget.value as PropertiesPlacement)}>
    <option value="top">At the top</option>
    <option value="left">At the left</option>
    <option value="right">At the right</option>
  </select>
</label>

<style>
  .sk-placement {
    display: inline-flex;
    align-items: center;
    gap: 0.3em;
    color: var(--sk-color-muted);
    white-space: nowrap;
  }

  select {
    font: inherit;
    color: inherit;
    background: var(--sk-color-ui-background);
    border: 1px solid var(--sk-color-border);
    border-radius: 3px;
  }

  /* The drop-down list is drawn by the webview; set its colors explicitly. */
  option {
    background: var(--sk-color-ui-background);
    color: var(--sk-color-foreground);
  }
</style>
