<script lang="ts">
  import { openProject, pickFolder } from "../ipc/project";
  import type { Package, Project } from "../ipc/types";
  import { shortcutFor, shortcutLabel, type Action } from "../keyboard";
  import { launchLabel, type Launch } from "../terminal/launch";
  import {
    scriptState,
    summarize,
    type ScriptState,
    type StatusSummary,
    type TerminalStatus,
  } from "../terminal/status";
  import Sidebar from "./Sidebar.svelte";
  import TabActions from "./TabActions.svelte";
  import TabBar, { type TabInfo } from "./TabBar.svelte";
  import TerminalPanel from "./TerminalPanel.svelte";

  interface Props {
    project: Project;
    /**
     * Whether this is the selected project. Hidden views stay mounted so their scripts keep
     * running; they ignore keyboard shortcuts.
     */
    active: boolean;
    /** Another folder was opened from this view (Ctrl+O); it is added, not swapped in. */
    onOpen: (project: Project) => void;
    /** Running and failed tab counts, for the project switcher. */
    onSummary: (summary: StatusSummary) => void;
  }

  let { project, active, onOpen, onSummary }: Props = $props();
  let openError = $state<string | null>(null);
  let sidebar: Sidebar;

  interface Tab extends TabInfo {
    launch: Launch;
  }

  // One tab per running (or finished) script, plus any number of shell tabs. Removing a tab
  // unmounts its terminal, which stops its process tree.
  let tabs = $state<Tab[]>([]);
  let activeKey = $state<number | null>(null);
  let nextKey = 0;
  const panels: Record<number, TerminalPanel> = {};

  function newTab(launch: Launch): Tab {
    nextKey += 1;
    const pkg = project.packages.find((p) => p.relativeDir === launch.packageDir);
    const owner = pkg?.name ?? (launch.packageDir || project.name);
    return { key: nextKey, launch, label: launchLabel(owner, launch), status: { kind: "starting" } };
  }

  /** ▶: focus the script's tab if it is still running, otherwise run it in a fresh tab. */
  function start(pkg: Package, script: string) {
    const index = tabs.findIndex(
      (t) => t.launch.kind === "script" && t.launch.packageDir === pkg.relativeDir && t.launch.script === script,
    );
    const existing = tabs[index];
    if (existing && (existing.status.kind === "starting" || existing.status.kind === "running")) {
      activeKey = existing.key;
      return;
    }
    const tab = newTab({ kind: "script", packageDir: pkg.relativeDir, script });
    if (existing) tabs[index] = tab;
    else tabs.push(tab);
    activeKey = tab.key;
  }

  /**
   * Opens the user's shell in a new tab, in the package folder (empty: the project root).
   * With `script`, its command line is typed at the prompt for the user to add arguments.
   */
  function openShellTab(packageDir: string, script: string | null = null) {
    const tab = newTab({ kind: "shell", packageDir, prefillScript: script });
    tabs.push(tab);
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

  // At most one script tab per script: ▶ reuses it (see `start`).
  const scriptStates = $derived(
    new Map(
      tabs.flatMap((t): [string, ScriptState][] =>
        t.launch.kind === "script" ? [[scriptKey(t.launch.packageDir, t.launch.script), scriptState(t.status)]] : [],
      ),
    ),
  );
  const activeTab = $derived(tabs.find((t) => t.key === activeKey));

  function scriptKey(packageDir: string, script: string): string {
    return `${packageDir}\0${script}`;
  }

  function stateOf(pkg: Package, script: string): ScriptState | null {
    return scriptStates.get(scriptKey(pkg.relativeDir, script)) ?? null;
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

  function run(action: Action) {
    const panel = activeKey === null ? undefined : panels[activeKey];
    switch (action) {
      case "open-folder":
        void openAnother();
        return;
      case "focus-filter":
        sidebar.focusFilter();
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
      case "open-shell":
        openShellTab("");
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
  <Sidebar
    bind:this={sidebar}
    {project}
    {stateOf}
    onRun={start}
    onShell={(pkg, script) => openShellTab(pkg.relativeDir, script)}
    error={openError}
    onDismissError={() => (openError = null)}
  />
  <main class:with-tabs={tabs.length > 0}>
    {#if tabs.length > 0}
      <TabBar
        {tabs}
        {activeKey}
        onSelect={(key) => (activeKey = key)}
        onClose={closeTab}
        onNewShell={() => openShellTab("")}
      >
        {#snippet actions()}
          {#if activeTab}
            {@const key = activeTab.key}
            <TabActions
              status={activeTab.status}
              kind={activeTab.launch.kind}
              onStop={() => void panels[key]?.stopScript()}
              onRestart={() => void panels[key]?.restartScript()}
            />
          {/if}
        {/snippet}
      </TabBar>
      <div class="panels">
        {#each tabs as tab (tab.key)}
          <div class="slot" hidden={tab.key !== activeKey}>
            <TerminalPanel
              bind:this={panels[tab.key]}
              projectRoot={project.root}
              launch={tab.launch}
              packageManager={project.packageManager}
              active={active && tab.key === activeKey}
              onStatus={(status) => setStatus(tab.key, status)}
            />
          </div>
        {/each}
      </div>
    {:else}
      <div class="placeholder">
        <p>Pick a script to run it here, or open a terminal.</p>
        <button onclick={() => openShellTab("")}>
          Open a terminal <kbd>{shortcutLabel("open-shell")}</kbd>
        </button>
      </div>
    {/if}
  </main>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(16rem, 22rem) minmax(0, 1fr);
    height: 100%;
  }

  main {
    min-width: 0;
    min-height: 0;
    background: var(--bg);
  }

  /* minmax(0, …): an `auto` column would grow to the terminal's and the tab strip's width. */
  main.with-tabs {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr);
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
    justify-items: center;
    gap: 0.75rem;
    height: 100%;
    color: var(--text-muted);
  }

  .placeholder p {
    margin: 0;
  }

  .placeholder button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
</style>
