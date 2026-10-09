<script lang="ts">
  import { formatReadingTime, formatWordCount } from "./reading";

  interface Props {
    wordCount: number;
    /** Title of the section at the top of the viewport. */
    section: string | null;
    /** Remote images in the document, and whether they're loading. */
    remoteImages: number;
    remoteImagesAllowed: boolean;
    onallowremote: () => void;
  }

  let { wordCount, section, remoteImages, remoteImagesAllowed, onallowremote }: Props = $props();
</script>

<footer class="sk-status">
  <span class="section" title={section ?? ""}>{section ?? ""}</span>
  {#if remoteImages > 0 && !remoteImagesAllowed}
    <span class="remote">
      {remoteImages} remote {remoteImages === 1 ? "image" : "images"} blocked
      <button onclick={onallowremote} title="Load remote images for this document">Load</button>
    </span>
  {/if}
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

  .remote button {
    font: inherit;
    color: var(--sk-color-link);
    background: none;
    border: 1px solid var(--sk-color-border);
    border-radius: 3px;
    padding: 0 0.4em;
    margin-left: 0.3em;
    cursor: pointer;
  }

  .section {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
