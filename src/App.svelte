<script lang="ts">
  import Home from "./lib/components/Home.svelte";
  import ProjectView from "./lib/components/ProjectView.svelte";
  import type { Project } from "./lib/ipc/types";

  let project = $state<Project | null>(null);
</script>

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
