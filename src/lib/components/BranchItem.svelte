<script lang="ts">
  import { headLabel, headMoved, rememberHead } from "../git";
  import { gitStatus } from "../ipc/git";
  import { refreshProject } from "../ipc/project";
  import type { GitStatus, Project } from "../ipc/types";
  import BranchMenu from "./BranchMenu.svelte";

  interface Props {
    root: string;
    /** The project was read again because its branch moved. */
    onProject: (project: Project) => void;
  }

  let { root, onProject }: Props = $props();

  let status = $state<GitStatus | null>(null);
  let open = $state(false);
  let wrapper = $state<HTMLElement>();
  // Answers to older requests (or for the previously shown project) are dropped.
  let request = 0;

  // No watchers or timers: the status is read when the project is shown, when the window
  // regains focus, when the menu opens, and after each action from the menu.
  $effect(() => {
    open = false;
    status = null;
    void refresh(root);
  });

  async function refresh(target: string) {
    const id = ++request;
    let next: GitStatus | null;
    try {
      next = await gitStatus(target);
    } catch {
      next = null;
    }
    if (id !== request) return;
    status = next;
    if (next && headMoved(target, next)) {
      try {
        onProject(await refreshProject(target));
      } catch {
        // The project was closed meanwhile, or no longer reads; its list stays as it was.
      }
    }
  }

  /** From the menu, after Runbook itself moved HEAD and already re-read the project. */
  function onStatus(target: string, next: GitStatus) {
    rememberHead(target, next);
    if (target === root) status = next;
  }

  function toggle() {
    open = !open;
    if (open) void refresh(root);
  }

  function onPointerDown(event: PointerEvent) {
    if (open && wrapper && !wrapper.contains(event.target as Node)) open = false;
  }

  function describe(s: GitStatus): string {
    const parts = [s.branch ? `Branch ${s.branch}` : `Detached at ${headLabel(s)}`];
    if (s.upstream) parts.push(`tracking ${s.upstream}: ${s.behind} behind, ${s.ahead} ahead`);
    return parts.join(", ");
  }
</script>

<svelte:window onfocus={() => void refresh(root)} onpointerdown={onPointerDown} />

{#if status}
  <div class="branch" bind:this={wrapper}>
    <button class="item" title={describe(status)} aria-expanded={open} onclick={toggle}>
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <circle cx="4.5" cy="3.5" r="1.6" />
        <circle cx="4.5" cy="12.5" r="1.6" />
        <circle cx="11.5" cy="5.5" r="1.6" />
        <path d="M4.5 5.1v5.8M11.5 7.1c0 2.4-2 3-5.5 4" />
      </svg>
      <bdi class="label">{headLabel(status)}</bdi>
      {#if status.behind > 0}<span>↓{status.behind}</span>{/if}
      {#if status.ahead > 0}<span>↑{status.ahead}</span>{/if}
    </button>
    {#if open}
      <BranchMenu {root} {status} {onStatus} {onProject} onClose={() => (open = false)} />
    {/if}
  </div>
  <span class="sep"></span>
{/if}

<style>
  .branch {
    position: relative;
    display: flex;
    min-width: 0;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
    max-width: 22rem;
    padding: 0.05rem 0.4rem;
    font: inherit;
    color: var(--text);
    background: transparent;
    border-color: transparent;
  }

  .item:hover,
  .item[aria-expanded="true"] {
    background: var(--hover);
  }

  svg {
    flex: 0 0 auto;
    width: 0.8rem;
    height: 0.8rem;
    fill: none;
    stroke: var(--text-muted);
    stroke-width: 1.3;
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sep {
    width: 1px;
    height: 0.75rem;
    background: var(--border);
  }
</style>
