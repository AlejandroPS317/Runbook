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
  let open = $state(false);
  let root: HTMLDivElement;
  let trigger: HTMLButtonElement;
  const current = $derived(projects.find((p) => p.root === activeRoot));

  function close(refocus: boolean) {
    open = false;
    if (refocus) trigger.focus();
  }

  function choose(action: () => void) {
    close(false);
    action();
  }

  function onWindowPointerDown(event: PointerEvent) {
    if (open && !root.contains(event.target as Node)) close(false);
  }

  // Capture phase, so the menu closes wherever focus is and the app's own Escape (leave the
  // home screen) and the terminal do not see the key as well.
  function onWindowKeydown(event: KeyboardEvent) {
    if (!open || event.key !== "Escape") return;
    event.stopPropagation();
    close(true);
  }
</script>

<svelte:window onpointerdown={onWindowPointerDown} onkeydowncapture={onWindowKeydown} />

{#snippet counts(summary: StatusSummary)}
  {#if summary.active > 0}
    <span class="count active" aria-label={`${summary.active} running`}>
      <span class="dot"></span>{summary.active}
    </span>
  {/if}
  {#if summary.failed > 0}
    <span class="count error" aria-label={`${summary.failed} failed`}>
      <span class="dot"></span>{summary.failed}
    </span>
  {/if}
{/snippet}

<div class="switcher" bind:this={root}>
  <button
    class="trigger"
    bind:this={trigger}
    aria-haspopup="menu"
    aria-expanded={open}
    title={current?.root ?? "Open projects"}
    onclick={() => (open = !open)}
  >
    <span class="folder" aria-hidden="true"></span>
    <span class="name">{current?.name ?? "Open projects"}</span>
    <span class="chevron" aria-hidden="true"></span>
  </button>
  {#if open}
    <div class="menu" role="menu">
      {#each projects as project, index (project.root)}
        {@const shortcut = projectShortcutLabel(index)}
        <div class="item" class:active={project.root === activeRoot}>
          <button class="select" role="menuitem" title={project.root} onclick={() => choose(() => onSelect(project.root))}>
            <span class="text">
              <span class="item-name">{project.name}</span>
              <span class="path">{project.root}</span>
            </span>
            {@render counts(project.summary)}
            {#if shortcut}<kbd>{shortcut}</kbd>{/if}
          </button>
          <button
            class="close"
            aria-label={`Close ${project.name} and stop its scripts`}
            title="Close the project and stop its scripts"
            onclick={() => onClose(project.root)}>×</button
          >
        </div>
      {/each}
      <hr />
      <button class="add" role="menuitem" onclick={() => choose(onAdd)}>Open another project…</button>
    </div>
  {/if}
</div>

<style>
  .switcher {
    position: relative;
    min-width: 0;
  }

  button {
    border: none;
    background: transparent;
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    max-width: 22rem;
    padding: 0.3rem 0.6rem;
    border: 1px solid var(--border);
    background: var(--raised);
  }

  /* A folder outline drawn with borders: a body and a small tab on top. */
  .folder {
    position: relative;
    flex: 0 0 auto;
    width: 0.8rem;
    height: 0.6rem;
    margin-top: 0.1rem;
    border: 1.5px solid var(--text-muted);
    border-radius: 2px;
  }

  .folder::before {
    content: "";
    position: absolute;
    top: -0.25rem;
    left: -1.5px;
    width: 0.35rem;
    height: 0.2rem;
    border: 1.5px solid var(--text-muted);
    border-bottom: none;
    border-radius: 2px 2px 0 0;
  }

  .name {
    font-family: var(--mono);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chevron {
    flex: 0 0 auto;
    width: 0.3rem;
    height: 0.3rem;
    margin: 0 0.1rem 0.15rem;
    border-right: 1.5px solid var(--text-muted);
    border-bottom: 1.5px solid var(--text-muted);
    transform: rotate(45deg);
  }

  .menu {
    position: absolute;
    top: calc(100% + 0.3rem);
    left: 0;
    z-index: 10;
    display: grid;
    width: max-content;
    min-width: 18rem;
    max-width: min(32rem, 90vw);
    padding: 0.3rem;
    background: var(--raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.25);
  }

  .item {
    display: flex;
    align-items: center;
    border-radius: var(--radius);
  }

  .item:hover {
    background: var(--hover);
  }

  .item.active {
    background: var(--accent-soft);
  }

  .select {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
    min-width: 0;
    padding: 0.35rem 0.5rem;
    text-align: left;
  }

  .select:hover:not(:disabled) {
    background: transparent;
  }

  .text {
    display: grid;
    flex: 1;
    min-width: 0;
  }

  .item-name {
    font-family: var(--mono);
    font-weight: 600;
  }

  .path {
    font-size: 0.75rem;
    color: var(--text-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .count {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }

  .dot {
    width: 0.45rem;
    height: 0.45rem;
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
    color: var(--text-faint);
    font-size: 1rem;
    line-height: 1;
  }

  .close:hover:not(:disabled) {
    color: var(--danger);
    background: transparent;
  }

  hr {
    margin: 0.3rem 0;
    border: none;
    border-top: 1px solid var(--border);
  }

  .add {
    padding: 0.4rem 0.5rem;
    text-align: left;
    color: var(--text-muted);
    border-radius: var(--radius);
  }

  .add:hover:not(:disabled) {
    color: var(--text);
    background: var(--hover);
  }
</style>
