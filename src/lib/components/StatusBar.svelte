<script lang="ts">
  import type { Project } from "../ipc/types";

  interface Props {
    project: Project;
  }

  let { project }: Props = $props();
  const scripts = $derived(project.packages.reduce((sum, p) => sum + p.scripts.length, 0));
  const problems = $derived(project.problems.length);

  function plural(count: number, noun: string): string {
    return `${count} ${noun}${count === 1 ? "" : "s"}`;
  }
</script>

<footer>
  <span class="dot" aria-hidden="true"></span>
  <span>{project.packageManager}</span>
  <span class="sep"></span>
  <span>{plural(project.packages.length, "package")}, {plural(scripts, "script")}</span>
  {#if problems > 0}
    <span class="sep"></span>
    <span class="problems">{plural(problems, "problem")}</span>
  {/if}
  <span class="path" title={project.root}><bdi>{project.root}</bdi></span>
</footer>

<style>
  footer {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
    padding: 0.3rem 0.75rem;
    font-family: var(--mono);
    font-size: 0.72rem;
    color: var(--text-muted);
    background: var(--surface);
    border-top: 1px solid var(--border);
    white-space: nowrap;
  }

  .dot {
    width: 0.4rem;
    height: 0.4rem;
    border-radius: 50%;
    background: var(--accent);
  }

  .sep {
    width: 1px;
    height: 0.75rem;
    background: var(--border);
  }

  .problems {
    color: var(--danger);
  }

  .path {
    flex: 1;
    min-width: 0;
    text-align: right;
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    /* Truncates from the left so the project folder stays visible; <bdi> keeps the
       leading "/" from moving to the end. */
    direction: rtl;
  }
</style>
