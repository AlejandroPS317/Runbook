<script lang="ts">
  import type { Package, Project } from "../ipc/types";
  import type { TerminalStatus } from "../terminal/status";
  import ScriptList from "./ScriptList.svelte";
  import TabBar, { type TabInfo } from "./TabBar.svelte";
  import TerminalPanel from "./TerminalPanel.svelte";

  interface Props {
    project: Project;
    onClose: () => void;
  }

  let { project, onClose }: Props = $props();

  interface Tab extends TabInfo {
    packageDir: string;
    script: string;
  }

  // One tab per running (or finished) script. Removing a tab unmounts its terminal, which
  // stops the script's process tree.
  let tabs = $state<Tab[]>([]);
  let activeKey = $state<number | null>(null);
  let nextKey = 0;

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
    if (activeKey === key) activeKey = (tabs[index] ?? tabs[index - 1])?.key ?? null;
  }

  function setStatus(key: number, status: TerminalStatus) {
    const tab = tabs.find((t) => t.key === key);
    if (tab) tab.status = status;
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
  <main class:with-tabs={tabs.length > 0}>
    {#if tabs.length > 0}
      <TabBar {tabs} {activeKey} onSelect={(key) => (activeKey = key)} onClose={closeTab} />
      <div class="panels">
        {#each tabs as tab (tab.key)}
          <div class="slot" hidden={tab.key !== activeKey}>
            <TerminalPanel
              packageDir={tab.packageDir}
              script={tab.script}
              label={tab.label}
              active={tab.key === activeKey}
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
