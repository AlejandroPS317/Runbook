<script lang="ts">
  import { filterPackages, inSection, scriptCount, type Section } from "../filter";
  import type { Package, Project } from "../ipc/types";
  import { shortcutLabel } from "../keyboard";
  import type { ScriptState } from "../terminal/status";
  import ErrorLine from "./ErrorLine.svelte";
  import ScriptList from "./ScriptList.svelte";

  interface Props {
    project: Project;
    stateOf: (pkg: Package, script: string) => ScriptState | null;
    onRun: (pkg: Package, script: string) => void;
    onShell: (pkg: Package, script: string | null) => void;
    error: string | null;
    onDismissError: () => void;
  }

  let { project, stateOf, onRun, onShell, error, onDismissError }: Props = $props();

  const sections: { id: Section; label: string }[] = [
    { id: "all", label: "All" },
    { id: "apps", label: "Apps" },
    { id: "packages", label: "Packages" },
  ];
  // Only monorepos laid out in apps/ and packages/ get the section switch.
  const sectioned = $derived(
    scriptCount(project.packages, "apps") + scriptCount(project.packages, "packages") > 0,
  );

  let query = $state("");
  let section = $state<Section>("all");
  let filterInput: HTMLInputElement;
  const visible = $derived(
    filterPackages(
      project.packages.filter((p) => inSection(p, section)),
      query,
    ),
  );
  // Same order as the list shows: workspace packages first, then the others.
  const firstVisible = $derived(
    [...visible.filter((p) => p.role !== "unlisted"), ...visible.filter((p) => p.role === "unlisted")].find(
      (p) => p.scripts.length > 0,
    ),
  );

  export function focusFilter() {
    filterInput.focus();
    filterInput.select();
  }

  function onFilterKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && firstVisible?.scripts[0]) {
      // Shift+Enter: type the command in a shell instead, to add arguments before running.
      if (event.shiftKey) onShell(firstVisible, firstVisible.scripts[0].name);
      else onRun(firstVisible, firstVisible.scripts[0].name);
    } else if (event.key === "Escape") {
      query = "";
      filterInput.blur();
    }
  }
</script>

<aside>
  {#if error}
    <ErrorLine message={error} onDismiss={onDismissError} />
  {/if}
  <div class="controls">
    <label class="filter">
      <span class="search" aria-hidden="true"></span>
      <input
        type="search"
        placeholder="Filter scripts or packages…"
        aria-label="Filter scripts"
        bind:value={query}
        bind:this={filterInput}
        onkeydown={onFilterKeydown}
      />
      <kbd>{shortcutLabel("focus-filter")}</kbd>
    </label>
    {#if sectioned}
      <div class="sections" role="radiogroup" aria-label="Show packages">
        {#each sections as item (item.id)}
          <button
            role="radio"
            aria-checked={section === item.id}
            class:selected={section === item.id}
            onclick={() => (section = item.id)}
          >
            {item.label}
            <span class="count">{scriptCount(project.packages, item.id)}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
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
  {#if visible.length === 0}
    <p class="no-match">
      {query ? `No scripts match “${query}”.` : "No scripts in this section."}
    </p>
  {:else}
    <ScriptList packages={visible} {stateOf} expandAll={query.trim() !== ""} {onRun} {onShell} />
  {/if}
</aside>

<style>
  aside {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--surface);
  }

  aside > :global(nav) {
    flex: 1;
    min-height: 0;
  }

  .controls {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0.5rem;
    padding: 0.6rem 0.6rem 0.5rem;
  }

  .filter {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0 0.4rem 0 0.55rem;
    background: var(--raised);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .filter:focus-within {
    border-color: var(--accent);
  }

  /* A magnifier drawn with borders: a ring and a short rotated handle. */
  .search {
    position: relative;
    flex: 0 0 auto;
    width: 0.5rem;
    height: 0.5rem;
    border: 1.5px solid var(--text-faint);
    border-radius: 50%;
  }

  .search::after {
    content: "";
    position: absolute;
    right: -0.2rem;
    bottom: -0.2rem;
    width: 0.25rem;
    height: 1.5px;
    background: var(--text-faint);
    transform: rotate(45deg);
  }

  input {
    flex: 1;
    width: 0;
    min-width: 0;
    padding: 0.4rem 0;
    font: inherit;
    color: inherit;
    background: transparent;
    border: none;
  }

  input:focus-visible {
    outline: none;
  }

  input::placeholder {
    color: var(--text-faint);
  }

  .sections {
    display: flex;
    gap: 0.25rem;
  }

  .sections button {
    padding: 0.2rem 0.55rem;
    font-size: 0.8rem;
    color: var(--text-muted);
    background: transparent;
    border-color: transparent;
  }

  .sections button.selected {
    color: var(--text);
    background: var(--raised);
    border-color: var(--border);
  }

  .count {
    margin-left: 0.15rem;
    font-size: 0.7rem;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }

  .no-match {
    margin: 0.5rem 0.75rem;
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .problems {
    margin: 0 0.6rem 0.5rem;
    padding: 0.4rem 0.6rem;
    color: var(--danger);
    background: var(--danger-soft);
    border-radius: var(--radius);
    font-size: 0.85rem;
  }

  .problems ul {
    margin: 0.4rem 0 0;
    padding-left: 1rem;
    color: var(--text);
  }
</style>
