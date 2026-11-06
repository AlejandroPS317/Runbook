<script lang="ts">
  import type { Package, Project } from "../ipc/types";
  import ScriptList from "./ScriptList.svelte";
  import TerminalPanel from "./TerminalPanel.svelte";

  interface Props {
    project: Project;
    onClose: () => void;
  }

  let { project, onClose }: Props = $props();

  interface Run {
    /** Changes on every run so the terminal remounts with a fresh session. */
    key: number;
    packageDir: string;
    script: string;
    label: string;
  }

  let run = $state<Run | null>(null);
  let nextKey = 0;

  function start(pkg: Package, script: string) {
    nextKey += 1;
    const owner = pkg.name ?? (pkg.relativeDir || project.name);
    run = { key: nextKey, packageDir: pkg.relativeDir, script, label: `${owner} › ${script}` };
  }
</script>

<div class="layout">
  <aside>
    <header>
      <div class="title">
        <strong>{project.name}</strong>
        <span class="muted">{project.packageManager}</span>
      </div>
      <button onclick={onClose}>Close</button>
    </header>
    {#if project.problems.length > 0}
      <details class="problems">
        <summary>{project.problems.length} problem{project.problems.length === 1 ? "" : "s"}</summary>
        <ul>
          {#each project.problems as problem (problem.path + problem.message)}
            <li><code>{problem.path || "."}</code> {problem.message}</li>
          {/each}
        </ul>
      </details>
    {/if}
    <ScriptList packages={project.packages} onRun={start} />
  </aside>
  <main>
    {#if run}
      {#key run.key}
        <TerminalPanel packageDir={run.packageDir} script={run.script} label={run.label} />
      {/key}
    {:else}
      <p class="placeholder muted">Pick a script to run it here.</p>
    {/if}
  </main>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(16rem, 22rem) 1fr;
    height: 100%;
  }

  aside {
    display: grid;
    grid-template-rows: auto auto 1fr;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--surface);
  }

  aside > header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.6rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .title {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    min-width: 0;
  }

  .title strong {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .muted {
    color: var(--text-muted);
  }

  .problems {
    padding: 0.4rem 0.75rem;
    border-bottom: 1px solid var(--border);
    color: var(--danger);
    font-size: 0.85rem;
  }

  .problems ul {
    margin: 0.4rem 0 0;
    padding-left: 1rem;
    color: var(--text);
  }

  main {
    min-width: 0;
    min-height: 0;
  }

  .placeholder {
    display: grid;
    place-content: center;
    height: 100%;
    margin: 0;
  }
</style>
