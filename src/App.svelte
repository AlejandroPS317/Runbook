<script lang="ts">
  import Home from "./lib/components/Home.svelte";
  import ProjectView from "./lib/components/ProjectView.svelte";
  import type { Project } from "./lib/ipc/types";

  let project = $state<Project | null>(null);
</script>

{#if project}
  <ProjectView {project} onClose={() => (project = null)} />
{:else}
  <Home onOpen={(opened) => (project = opened)} />
{/if}
