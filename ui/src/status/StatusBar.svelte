<script lang="ts">
  import { formatReadingTime, formatWordCount } from "./reading";

  interface Props {
    wordCount: number;
    /** Title of the section at the top of the viewport. */
    section: string | null;
  }

  let { wordCount, section }: Props = $props();
</script>

<footer class="sk-status">
  <span class="section" title={section ?? ""}>{section ?? ""}</span>
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

  .section {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
