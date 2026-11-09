<script lang="ts">
  import type { Project } from "../ipc/types";
  import ProjectSwitcher, { type ProjectEntry } from "./ProjectSwitcher.svelte";

  interface Props {
    projects: ProjectEntry[];
    /** `null` while the home screen is shown. */
    activeRoot: string | null;
    packageManager: Project["packageManager"] | null;
    onSelect: (root: string) => void;
    onClose: (root: string) => void;
    onAdd: () => void;
  }

  let { projects, activeRoot, packageManager, onSelect, onClose, onAdd }: Props = $props();
  const running = $derived(projects.reduce((sum, p) => sum + p.summary.active, 0));
  const failed = $derived(projects.reduce((sum, p) => sum + p.summary.failed, 0));
</script>

<header>
  <ProjectSwitcher {projects} {activeRoot} {onSelect} {onClose} {onAdd} />
  <button class="add" aria-label="Open another project" title="Open another project" onclick={onAdd}>+</button>
  <div class="spacer"></div>
  <div class="totals" title="Scripts and shells across every open project">
    <span class="total" class:on={running > 0}><span class="dot"></span>{running} running</span>
    {#if failed > 0}
      <span class="sep"></span>
      <span class="total error"><span class="dot"></span>{failed} failed</span>
    {/if}
  </div>
  {#if packageManager}
    <span class="pm" title="Package manager of this project">{packageManager}</span>
  {/if}
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    padding: 0.45rem 0.6rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .add {
    flex: 0 0 auto;
    padding: 0.2rem 0.55rem;
    color: var(--text-muted);
    font-size: 1rem;
    line-height: 1.2;
  }

  .spacer {
    flex: 1;
  }

  .totals {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex: 0 0 auto;
    padding: 0.25rem 0.65rem;
    font-family: var(--mono);
    font-size: 0.75rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .total {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .total.on {
    color: var(--text);
  }

  .dot {
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 50%;
    background: var(--text-faint);
  }

  .total.on .dot {
    background: var(--accent);
  }

  .total.error {
    color: var(--danger);
  }

  .total.error .dot {
    background: var(--danger);
  }

  .sep {
    width: 1px;
    height: 0.8rem;
    background: var(--border);
  }

  .pm {
    flex: 0 0 auto;
    padding: 0.25rem 0.55rem;
    font-family: var(--mono);
    font-size: 0.75rem;
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
</style>
