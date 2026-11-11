<script lang="ts">
  import { onMount } from "svelte";
  import { filterBranches } from "../git";
  import { gitBranches, gitFetch, gitPull, gitStatus, gitSwitch } from "../ipc/git";
  import type { Branch, GitChange, GitStatus, Project } from "../ipc/types";
  import BranchList from "./BranchList.svelte";

  interface Props {
    root: string;
    status: GitStatus;
    /** Reported with the root it belongs to: the visible project may change meanwhile. */
    onStatus: (root: string, status: GitStatus) => void;
    onProject: (project: Project) => void;
    onClose: () => void;
  }

  let { root, status, onStatus, onProject, onClose }: Props = $props();

  let branches = $state<Branch[]>([]);
  let loaded = $state(false);
  let query = $state("");
  let selected = $state(0);
  let busy = $state<"switch" | "fetch" | "pull" | null>(null);
  let error = $state<string | null>(null);
  /** A switch waiting for the user to accept that uncommitted changes come along. */
  let confirm = $state<{ branch: Branch; changes: number } | null>(null);
  let input: HTMLInputElement;

  const visible = $derived(filterBranches(branches, query));
  const canPull = $derived(status.branch !== null && status.upstream !== null);

  onMount(() => {
    input.focus();
    void load();
  });

  async function load() {
    try {
      branches = await gitBranches(root);
    } catch (err) {
      error = String(err);
    } finally {
      loaded = true;
    }
  }

  /** Runs one Git action at a time; failures show Git's message in the menu. */
  async function run<T>(kind: "switch" | "fetch" | "pull", task: () => Promise<T>): Promise<T | undefined> {
    busy = kind;
    error = null;
    try {
      return await task();
    } catch (err) {
      error = String(err);
      return undefined;
    } finally {
      busy = null;
    }
  }

  async function choose(branch: Branch) {
    if (branch.current || busy !== null) return;
    confirm = null;
    // Fresh count: files may have changed since the menu opened.
    const fresh = await run("switch", () => gitStatus(root));
    if (fresh === undefined) return;
    if (fresh) onStatus(root, fresh);
    if (fresh && fresh.changes > 0) confirm = { branch, changes: fresh.changes };
    else await switchTo(branch);
  }

  async function switchTo(branch: Branch) {
    confirm = null;
    const change = await run("switch", () => gitSwitch(root, branch));
    if (change && apply(change)) onClose();
  }

  async function fetchRemotes() {
    const next = await run("fetch", () => gitFetch(root));
    if (!next) return;
    onStatus(root, next);
    await load();
  }

  async function pull() {
    const change = await run("pull", () => gitPull(root));
    if (change) apply(change);
  }

  /** Returns whether the scripts were read again; the Git action happened either way. */
  function apply(change: GitChange): boolean {
    onStatus(root, change.status);
    if (change.project) onProject(change.project);
    if (change.discoveryError === null) return true;
    error = `The branch changed, but the scripts could not be read again: ${change.discoveryError}`;
    return false;
  }

  function onKeydown(event: KeyboardEvent) {
    const target = visible[selected];
    if (event.key === "ArrowDown") selected = Math.max(0, Math.min(selected + 1, visible.length - 1));
    else if (event.key === "ArrowUp") selected = Math.max(selected - 1, 0);
    else if (event.key === "Enter" && target) void choose(target);
    else if (event.key === "Escape") {
      if (confirm) confirm = null;
      else onClose();
    } else return;
    // Keeps project shortcuts and the app's Escape handling out of the menu.
    event.preventDefault();
    event.stopPropagation();
  }

  function plural(count: number, noun: string): string {
    return `${count} ${noun}${count === 1 ? "" : "s"}`;
  }
</script>

<div class="menu" role="dialog" aria-label="Branches">
  <div class="top">
    <input
      type="search"
      placeholder="Filter branches…"
      aria-label="Filter branches"
      bind:value={query}
      bind:this={input}
      oninput={() => (selected = 0)}
      onkeydown={onKeydown}
    />
    <button disabled={busy !== null} class:busy={busy === "fetch"} title="Fetch every remote" onclick={fetchRemotes}>
      Fetch
    </button>
    <button
      disabled={busy !== null || !canPull}
      class:busy={busy === "pull"}
      title={canPull ? "Fast-forward to the upstream branch" : "This branch has no upstream"}
      onclick={pull}
    >
      Pull
    </button>
  </div>
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  {#if confirm}
    <div class="confirm">
      <span>
        {plural(confirm.changes, "changed file")} will move to <bdi>{confirm.branch.name}</bdi>.
      </span>
      <button class="primary" onclick={() => confirm && switchTo(confirm.branch)}>Switch</button>
      <button onclick={() => (confirm = null)}>Cancel</button>
    </div>
  {/if}
  <BranchList
    branches={visible}
    {selected}
    busy={busy !== null}
    empty={!loaded ? "Loading branches…" : query ? `No branches match “${query}”.` : "No branches."}
    onChoose={choose}
    onHover={(i) => (selected = i)}
  />
</div>

<style>
  .menu {
    position: absolute;
    bottom: calc(100% + 0.35rem);
    left: 0;
    z-index: 10;
    display: flex;
    flex-direction: column;
    width: min(26rem, 90vw);
    max-height: 60vh;
    font-family: var(--sans);
    font-size: 0.85rem;
    color: var(--text);
    white-space: normal;
    background: var(--raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  }

  .top {
    display: flex;
    gap: 0.35rem;
    padding: 0.5rem;
    border-bottom: 1px solid var(--border);
  }

  input {
    flex: 1;
    min-width: 0;
    padding: 0.3rem 0.5rem;
    font: inherit;
    color: inherit;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  input:focus-visible {
    outline: none;
    border-color: var(--accent);
  }

  .top button {
    padding: 0.2rem 0.6rem;
    font-size: 0.8rem;
  }

  .busy {
    cursor: progress;
    color: var(--accent);
  }

  .error,
  .confirm {
    margin: 0.5rem 0.5rem 0;
    padding: 0.4rem 0.6rem;
    border-radius: var(--radius);
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }

  .error {
    color: var(--danger);
    background: var(--danger-soft);
  }

  .confirm {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
    background: var(--accent-soft);
  }

  .confirm span {
    flex: 1 1 12rem;
  }

  .primary {
    color: var(--accent-contrast);
    background: var(--accent);
    border-color: var(--accent);
  }

</style>
