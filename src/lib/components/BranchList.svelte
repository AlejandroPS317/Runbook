<script lang="ts">
  import { branchKey } from "../git";
  import type { Branch } from "../ipc/types";

  interface Props {
    branches: Branch[];
    /** Index of the keyboard selection in `branches`. */
    selected: number;
    busy: boolean;
    /** Shown when `branches` is empty. */
    empty: string;
    onChoose: (branch: Branch) => void;
    onHover: (index: number) => void;
  }

  let { branches, selected, busy, empty, onChoose, onHover }: Props = $props();
  const rows: HTMLElement[] = [];

  $effect(() => {
    rows[selected]?.scrollIntoView({ block: "nearest" });
  });
</script>

<ul aria-busy={busy}>
  {#each branches as branch, i (branchKey(branch))}
    {#if branch.remote !== null && (i === 0 || branches[i - 1]?.remote === null)}
      <li class="group">Remote</li>
    {/if}
    <li>
      <button
        bind:this={rows[i]}
        class="row"
        class:selected={i === selected}
        class:current={branch.current}
        disabled={busy}
        onclick={() => onChoose(branch)}
        onpointermove={() => onHover(i)}
      >
        <span class="check" aria-hidden="true">{branch.current ? "✓" : ""}</span>
        <bdi class="name">{branch.name}</bdi>
        {#if branch.remote}<span class="remote">{branch.remote}</span>{/if}
      </button>
    </li>
  {:else}
    <li class="empty">{empty}</li>
  {/each}
</ul>

<style>
  ul {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: 0.35rem;
    overflow-y: auto;
    list-style: none;
  }

  ul[aria-busy="true"] {
    cursor: progress;
  }

  .group {
    padding: 0.5rem 0.5rem 0.2rem;
    font-size: 0.72rem;
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    width: 100%;
    padding: 0.3rem 0.5rem;
    text-align: left;
    background: transparent;
    border-color: transparent;
  }

  .row.selected {
    background: var(--hover);
  }

  .row.current {
    font-weight: 600;
  }

  .check {
    flex: 0 0 1rem;
    color: var(--accent);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
    font-size: 0.8rem;
  }

  .remote {
    flex: 0 0 auto;
    font-size: 0.72rem;
    color: var(--text-faint);
  }

  .empty {
    padding: 0.5rem;
    color: var(--text-muted);
  }
</style>
