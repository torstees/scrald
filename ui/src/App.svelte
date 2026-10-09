<script lang="ts">
  import { onMount } from "svelte";
  import { launchInfo } from "./lib/commands";
  import { launchMessage } from "./lib/launch";
  import type { LaunchInfo } from "./lib/types";

  let info = $state<LaunchInfo | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      info = await launchInfo();
    } catch (e) {
      error = String(e);
    }
  });
</script>

<main>
  <h1>{info?.title ?? "Scrald"}</h1>
  {#if error}
    <p class="status">Could not reach the Scrald backend: {error}</p>
  {:else if info}
    <p class="status">{launchMessage(info)}</p>
  {/if}
</main>

<style>
  main {
    max-width: 66ch;
    margin: 0 auto;
    padding: 3rem 1.5rem;
  }

  .status {
    color: var(--sk-color-muted);
    font-family: var(--sk-font-ui);
    font-size: 0.9rem;
    overflow-wrap: anywhere;
  }
</style>
