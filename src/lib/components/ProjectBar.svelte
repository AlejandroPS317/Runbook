<script module lang="ts">
  import type { StatusSummary } from "../terminal/status";

  export interface ProjectEntry {
    root: string;
    name: string;
    summary: StatusSummary;
  }
</script>

<script lang="ts">
  import { projectShortcutLabel } from "../keyboard";

  interface Props {
    projects: ProjectEntry[];
    /** `null` while the home screen is shown. */
    activeRoot: string | null;
    onSelect: (root: string) => void;
    onClose: (root: string) => void;
    onAdd: () => void;
  }

  let { projects, activeRoot, onSelect, onClose, onAdd }: Props = $props();

  function title(project: ProjectEntry, index: number): string {
    const shortcut = projectShortcutLabel(index);
    const counts = [
      project.summary.active > 0 ? `${project.summary.active} running` : null,
      project.summary.failed > 0 ? `${project.summary.failed} failed` : null,
    ].filter((part) => part !== null);
    return [project.root, counts.join(", "), shortcut].filter(Boolean).join(" — ");
  }
</script>

<div class="projects" role="tablist" aria-label="Open projects">
  {#each projects as project, index (project.root)}
    <div class="project" class:active={project.root === activeRoot}>
      <button
        class="select"
        role="tab"
        aria-selected={project.root === activeRoot}
        title={title(project, index)}
        onclick={() => onSelect(project.root)}
      >
        <span class="name">{project.name}</span>
        {#if project.summary.active > 0}
          <span class="count active" aria-label={`${project.summary.active} running`}>
            <span class="dot"></span>{project.summary.active}
          </span>
        {/if}
        {#if project.summary.failed > 0}
          <span class="count error" aria-label={`${project.summary.failed} failed`}>
            <span class="dot"></span>{project.summary.failed}
          </span>
        {/if}
      </button>
      <button
        class="close"
        aria-label={`Close ${project.name} and stop its scripts`}
        title="Close the project and stop its scripts"
        onclick={() => onClose(project.root)}>×</button
      >
    </div>
  {/each}
  <button
    class="add"
    class:active={activeRoot === null}
    aria-label="Open another project"
    title="Open another project"
    onclick={onAdd}>+</button
  >
</div>

<style>
  .projects {
    display: flex;
    overflow-x: auto;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  /* Same classic scrollbar as the script tab strip; see TabBar.svelte for why. */
  .projects::-webkit-scrollbar {
    height: 6px;
  }

  .projects::-webkit-scrollbar-track {
    background: transparent;
  }

  .projects::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 3px;
  }

  .project {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    max-width: 18rem;
    border-right: 1px solid var(--border);
  }

  .project.active {
    background: var(--bg);
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  button {
    border: none;
    border-radius: 0;
    background: transparent;
  }

  .select {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    padding: 0.45rem 0.25rem 0.45rem 0.85rem;
  }

  .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .project:not(.active) .name {
    color: var(--text-muted);
  }

  .count {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }

  .dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
  }

  .count.active .dot {
    background: var(--accent);
  }

  .count.error {
    color: var(--danger);
  }

  .count.error .dot {
    background: var(--danger);
  }

  .close {
    padding: 0.2rem 0.5rem;
    color: var(--text-muted);
    font-size: 1rem;
    line-height: 1;
  }

  .close:hover,
  .add:hover {
    color: var(--text);
  }

  .add {
    flex: 0 0 auto;
    padding: 0.2rem 0.8rem;
    color: var(--text-muted);
    font-size: 1.1rem;
  }

  .add.active {
    color: var(--text);
    box-shadow: inset 0 -2px 0 var(--accent);
  }
</style>
