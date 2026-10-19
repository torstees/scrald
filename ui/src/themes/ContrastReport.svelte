<script lang="ts">
  // A theme's contrast check (DESIGN.md §7.2): passes, or the color pairs
  // below WCAG AA with a sample of each, worst first.
  import type { ContrastProblem } from "../lib/types";
  import { colorLabel, formatRatio } from "./contrast";

  interface Props {
    problems: ContrastProblem[];
  }

  let { problems }: Props = $props();
</script>

<div class="sk-contrast" role="status">
  {#if problems.length === 0}
    <p>✓ Meets WCAG AA contrast: 4.5:1 for text, 3:1 for the accent.</p>
  {:else}
    <p>
      {problems.length} color pair{problems.length === 1 ? "" : "s"} below WCAG AA contrast (4.5:1 for text, 3:1 for the accent):
    </p>
    <ul>
      {#each problems as p (p.what)}
        <li>
          <span class="sample" style:color={p.fg} style:background={p.bg} aria-hidden="true">Aa</span>
          <span class="what">
            {p.what}
            <span class="colors">{p.colors.map(colorLabel).join(" on ")}</span>
          </span>
          <span class="ratio">{formatRatio(p.ratio)} <span class="needs">needs {p.minimum}:1</span></span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .sk-contrast p {
    margin: 0.2rem 0 0.4rem;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  /* The pair as drawn; its colors come from the theme being checked. */
  .sample {
    flex: 0 0 auto;
    padding: 0 0.35rem;
    border: 1px solid var(--sk-color-border);
    border-radius: 3px;
    font-family: Georgia, serif;
  }

  .what {
    flex: 1;
    min-width: 0;
  }

  .colors {
    display: block;
    color: var(--sk-color-muted);
    font-size: 0.72rem;
  }

  .ratio {
    flex: 0 0 auto;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .needs {
    display: block;
    color: var(--sk-color-muted);
    font-size: 0.72rem;
  }
</style>
