<script lang="ts">
  import { filterPackages } from "../filter";
  import { openProject, pickFolder } from "../ipc/project";
  import type { Package, Project } from "../ipc/types";
  import { shortcutFor, shortcutLabel, type Action } from "../keyboard";
  import { summarize, type StatusSummary, type TerminalStatus } from "../terminal/status";
  import ErrorLine from "./ErrorLine.svelte";
  import ScriptList from "./ScriptList.svelte";
  import TabBar, { type TabInfo } from "./TabBar.svelte";
  import TerminalPanel from "./TerminalPanel.svelte";

  interface Props {
    project: Project;
    /**
     * Whether this is the selected project. Hidden views stay mounted so their scripts keep
     * running; they ignore keyboard shortcuts.
     */
    active: boolean;
    onClose: () => void;
    /** Another folder was opened from this view (Ctrl+O); it is added, not swapped in. */
    onOpen: (project: Project) => void;
    /** Running and failed tab counts, for the project bar. */
    onSummary: (summary: StatusSummary) => void;
  }

  let { project, active, onClose, onOpen, onSummary }: Props = $props();
  let openError = $state<string | null>(null);

  let query = $state("");
  let filterInput: HTMLInputElement;
  const visible = $derived(filterPackages(project.packages, query));
  // Same order as the list shows: workspace packages first, then the others.
  const firstVisible = $derived(
    [...visible.filter((p) => p.role !== "unlisted"), ...visible.filter((p) => p.role === "unlisted")].find(
      (p) => p.scripts.length > 0,
    ),
  );

  interface Tab extends TabInfo {
    packageDir: string;
    script: string;
  }

  // One tab per running (or finished) script. Removing a tab unmounts its terminal, which
  // stops the script's process tree.
  let tabs = $state<Tab[]>([]);
  let activeKey = $state<number | null>(null);
  let nextKey = 0;
  const panels: Record<number, TerminalPanel> = {};

  function newTab(pkg: Package, script: string): Tab {
    nextKey += 1;
    const owner = pkg.name ?? (pkg.relativeDir || project.name);
    return {
      key: nextKey,
      packageDir: pkg.relativeDir,
      script,
      label: `${owner} › ${script}`,
      status: { kind: "starting" },
    };
  }

  /** ▶: focus the script's tab if it is still running, otherwise run it in a fresh tab. */
  function start(pkg: Package, script: string) {
    const index = tabs.findIndex((t) => t.packageDir === pkg.relativeDir && t.script === script);
    const existing = tabs[index];
    if (existing && (existing.status.kind === "starting" || existing.status.kind === "running")) {
      activeKey = existing.key;
      return;
    }
    const tab = newTab(pkg, script);
    if (existing) tabs[index] = tab;
    else tabs.push(tab);
    activeKey = tab.key;
  }

  function closeTab(key: number) {
    const index = tabs.findIndex((t) => t.key === key);
    if (index === -1) return;
    tabs.splice(index, 1);
    delete panels[key];
    if (activeKey === key) activeKey = (tabs[index] ?? tabs[index - 1])?.key ?? null;
  }

  function setStatus(key: number, status: TerminalStatus) {
    const tab = tabs.find((t) => t.key === key);
    if (tab) tab.status = status;
  }

  function cycleTab(step: number) {
    if (tabs.length === 0) return;
    const index = tabs.findIndex((t) => t.key === activeKey);
    const next = tabs[(index + step + tabs.length) % tabs.length];
    if (next) activeKey = next.key;
  }

  $effect(() => onSummary(summarize(tabs.map((t) => t.status))));

  async function openAnother() {
    const path = await pickFolder();
    if (!path) return;
    openError = null;
    try {
      onOpen(await openProject(path));
    } catch (err) {
      openError = String(err);
    }
  }

  function onFilterKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && firstVisible?.scripts[0]) {
      start(firstVisible, firstVisible.scripts[0].name);
    } else if (event.key === "Escape") {
      query = "";
      filterInput.blur();
    }
  }

  function run(action: Action) {
    const panel = activeKey === null ? undefined : panels[activeKey];
    switch (action) {
      case "open-folder":
        void openAnother();
        return;
      case "focus-filter":
        filterInput.focus();
        filterInput.select();
        return;
      case "next-tab":
        cycleTab(1);
        return;
      case "previous-tab":
        cycleTab(-1);
        return;
      case "close-tab":
        if (activeKey !== null) closeTab(activeKey);
        return;
      case "restart-tab":
        void panel?.restartScript();
        return;
      case "stop-tab":
        void panel?.stopScript();
        return;
      default: {
        const unreachable: never = action;
        return unreachable;
      }
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (!active) return;
    const action = shortcutFor(event);
    if (action === null) return;
    event.preventDefault();
    run(action);
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="layout">
  <aside>
    <header>
      <div class="title">
        <strong>{project.name}</strong>
        <span class="muted">{project.packageManager}</span>
      </div>
      <button onclick={onClose} title="Close the project and stop its scripts">Close</button>
    </header>
    {#if openError}
      <ErrorLine message={openError} onDismiss={() => (openError = null)} />
    {/if}
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
    <input
      class="filter"
      type="search"
      placeholder={`Filter scripts (${shortcutLabel("focus-filter")})`}
      aria-label="Filter scripts"
      bind:value={query}
      bind:this={filterInput}
      onkeydown={onFilterKeydown}
    />
    {#if visible.length === 0}
      <p class="no-match muted">No scripts match “{query}”.</p>
    {:else}
      <ScriptList packages={visible} onRun={start} />
    {/if}
  </aside>
  <main class:with-tabs={tabs.length > 0}>
    {#if tabs.length > 0}
      <TabBar {tabs} {activeKey} onSelect={(key) => (activeKey = key)} onClose={closeTab} />
      <div class="panels">
        {#each tabs as tab (tab.key)}
          <div class="slot" hidden={tab.key !== activeKey}>
            <TerminalPanel
              bind:this={panels[tab.key]}
              projectRoot={project.root}
              packageDir={tab.packageDir}
              script={tab.script}
              label={tab.label}
              active={active && tab.key === activeKey}
              onStatus={(status) => setStatus(tab.key, status)}
            />
          </div>
        {/each}
      </div>
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
    display: flex;
    flex-direction: column;
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

  aside > :global(nav) {
    flex: 1;
    min-height: 0;
  }

  .filter {
    margin: 0.5rem 0.75rem 0.25rem;
    padding: 0.35rem 0.5rem;
    font: inherit;
    color: inherit;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .filter:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .no-match {
    margin: 0.5rem 0.75rem;
    font-size: 0.85rem;
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

  main.with-tabs {
    display: grid;
    grid-template-rows: auto 1fr;
  }

  .panels,
  .slot {
    min-height: 0;
    height: 100%;
  }

  .slot[hidden] {
    display: none;
  }

  .placeholder {
    display: grid;
    place-content: center;
    height: 100%;
    margin: 0;
  }
</style>
