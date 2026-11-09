<script lang="ts">
  import { onMount } from "svelte";
  import Home from "./lib/components/Home.svelte";
  import ProjectView from "./lib/components/ProjectView.svelte";
  import UpdateBanner from "./lib/components/UpdateBanner.svelte";
  import type { Project } from "./lib/ipc/types";
  import { checkForUpdatesAtStartup } from "./lib/settings";
  import { updates } from "./lib/updates.svelte";

  let project = $state<Project | null>(null);

  onMount(() => {
    // Development builds have no installed bundle to replace, so only manual checks run.
    if (!import.meta.env.DEV && checkForUpdatesAtStartup()) void updates.check(false);
  });
</script>

<div class="app">
  <UpdateBanner />
  <div class="content">
    {#if project}
      <!-- A new project remounts the view, which closes (and stops) the old project's tabs. -->
      {#key project.root}
        <ProjectView
          {project}
          onClose={() => (project = null)}
          onReplace={(opened) => (project = opened)}
        />
      {/key}
    {:else}
      <Home onOpen={(opened) => (project = opened)} />
    {/if}
  </div>
</div>

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr;
    height: 100%;
  }

  /* Explicit row: without a banner the content would otherwise take the `auto` row. */
  .content {
    grid-row: 2;
    min-height: 0;
  }
</style>
