<script lang="ts">
  import { onMount } from "svelte";
  import Home from "./lib/components/Home.svelte";
  import ProjectBar, { type ProjectEntry } from "./lib/components/ProjectBar.svelte";
  import ProjectView from "./lib/components/ProjectView.svelte";
  import UpdateBanner from "./lib/components/UpdateBanner.svelte";
  import { closeProject } from "./lib/ipc/project";
  import type { Project } from "./lib/ipc/types";
  import { projectIndexFor } from "./lib/keyboard";
  import { checkForUpdatesAtStartup } from "./lib/settings";
  import type { StatusSummary } from "./lib/terminal/status";
  import { updates } from "./lib/updates.svelte";

  // Every open project stays mounted while hidden: unmounting a view stops its scripts.
  let projects = $state<Project[]>([]);
  let summaries = $state<Record<string, StatusSummary>>({});
  // Kept while the home screen is shown, so Escape returns to it.
  let selectedRoot = $state<string | null>(null);
  let homeRequested = $state(false);
  const showHome = $derived(projects.length === 0 || homeRequested);
  const entries = $derived<ProjectEntry[]>(
    projects.map((p) => ({
      root: p.root,
      name: p.name,
      summary: summaries[p.root] ?? { active: 0, failed: 0 },
    })),
  );

  onMount(() => {
    // Development builds have no installed bundle to replace, so only manual checks run.
    if (!import.meta.env.DEV && checkForUpdatesAtStartup()) void updates.check(false);
  });

  /** Adds the project, or selects it when it is already open (its view is left as is). */
  function open(project: Project) {
    if (!projects.some((p) => p.root === project.root)) projects.push(project);
    select(project.root);
  }

  function select(root: string) {
    selectedRoot = root;
    homeRequested = false;
  }

  function close(root: string) {
    const index = projects.findIndex((p) => p.root === root);
    if (index === -1) return;
    projects.splice(index, 1);
    delete summaries[root];
    if (selectedRoot === root) selectedRoot = (projects[index] ?? projects[index - 1])?.root ?? null;
    // The backend stops the project's scripts even if a terminal is still mid-start.
    void closeProject(root);
  }

  function onKeydown(event: KeyboardEvent) {
    const index = projectIndexFor(event);
    if (index !== null) {
      const project = projects[index];
      if (project) {
        event.preventDefault();
        select(project.root);
      }
    } else if (event.key === "Escape" && homeRequested && projects.length > 0) {
      event.preventDefault();
      homeRequested = false;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="app">
  <UpdateBanner />
  <div class="content">
    {#if projects.length > 0}
      <ProjectBar
        projects={entries}
        activeRoot={showHome ? null : selectedRoot}
        onSelect={select}
        onClose={close}
        onAdd={() => (homeRequested = true)}
      />
    {/if}
    <div class="views">
      {#each projects as project (project.root)}
        {@const active = !showHome && project.root === selectedRoot}
        <div class="slot" hidden={!active}>
          <ProjectView
            {project}
            {active}
            onClose={() => close(project.root)}
            onOpen={open}
            onSummary={(summary) => {
              summaries[project.root] = summary;
            }}
          />
        </div>
      {/each}
      {#if showHome}
        <Home onOpen={open} />
      {/if}
    </div>
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
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-height: 0;
  }

  /* Explicit row for the same reason, when no project bar is shown. */
  .views {
    grid-row: 2;
    min-height: 0;
  }

  .slot {
    height: 100%;
  }

  .slot[hidden] {
    display: none;
  }
</style>
