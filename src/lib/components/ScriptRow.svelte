<script lang="ts">
  import type { Script } from "../ipc/types";
  import type { ScriptState } from "../terminal/status";

  interface Props {
    script: Script;
    /** Package name shown in labels. */
    owner: string;
    /** State of the script's tab, or `null` when it has none. */
    state: ScriptState | null;
    onRun: () => void;
    /** Opens a shell in the package folder with the script's command typed at the prompt. */
    onShell: () => void;
  }

  let { script, owner, state, onRun, onShell }: Props = $props();
</script>

<li class="row {state ?? 'idle'}">
  <button
    class="run"
    title={state === "running" ? "Show its terminal" : `Run: ${script.command}`}
    aria-label={`Run ${script.name} in ${owner}`}
    onclick={onRun}
  >
    <span class="icon" aria-hidden="true"></span>
    <span class="script">{script.name}</span>
    {#if state === null}
      <span class="command">{script.command}</span>
    {:else}
      <span class="state">{state}</span>
    {/if}
  </button>
  <button
    class="shell"
    title="Open in a terminal with the command typed, to add arguments"
    aria-label={`Open ${script.name} in a terminal in ${owner}`}
    onclick={onShell}>&gt;_</button
  >
</li>

<style>
  button {
    border: none;
    background: transparent;
  }

  .row {
    display: flex;
    align-items: center;
    margin: 1px 0;
    border: 1px solid transparent;
    border-radius: var(--radius);
  }

  .row:hover {
    background: var(--hover);
  }

  .row.running {
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  }

  .run {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    flex: 1;
    min-width: 0;
    padding: 0.3rem 0.4rem 0.3rem 1.1rem;
    text-align: left;
  }

  .run:hover:not(:disabled) {
    background: transparent;
  }

  .icon {
    flex: 0 0 auto;
    display: grid;
    place-items: center;
    width: 0.7rem;
    color: var(--text-faint);
    font-size: 0.75rem;
    line-height: 1;
  }

  .row.idle .icon::before {
    content: "";
    width: 0.45rem;
    height: 0.55rem;
    background: currentColor;
    clip-path: polygon(0 0, 100% 50%, 0 100%);
  }

  .row.idle:hover .icon {
    color: var(--accent);
  }

  .row.running .icon::before {
    content: "";
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 50%;
    background: var(--accent);
  }

  .row.passed .icon::before {
    content: "✓";
    color: var(--accent);
  }

  .row.failed .icon::before {
    content: "✕";
    color: var(--danger);
  }

  .row.stopped .icon::before {
    content: "";
    width: 0.4rem;
    height: 0.4rem;
    background: currentColor;
  }

  .script {
    flex: 0 1 auto;
    min-width: 0;
    font-family: var(--mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .command {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
    font-size: 0.7rem;
    color: var(--text-faint);
    text-align: right;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .state {
    margin-left: auto;
    font-family: var(--mono);
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  .row.running .state {
    padding: 0.05rem 0.4rem;
    color: var(--accent);
    border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
    border-radius: 4px;
  }

  .row.passed .state {
    color: var(--accent);
  }

  .row.failed .state {
    color: var(--danger);
  }

  /* Shown on hover or keyboard focus, so the list stays as quiet as a plain script list. */
  .shell {
    flex: 0 0 auto;
    padding: 0.1rem 0.35rem;
    margin-right: 0.15rem;
    font-family: var(--mono);
    font-size: 0.7rem;
    color: var(--text-muted);
    opacity: 0;
  }

  .row:hover .shell,
  .shell:focus-visible {
    opacity: 1;
  }

  .shell:hover:not(:disabled) {
    color: var(--text);
    background: var(--hover);
  }
</style>
