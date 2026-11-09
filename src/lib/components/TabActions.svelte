<script lang="ts">
  import { shortcutLabel } from "../keyboard";
  import type { Launch } from "../terminal/launch";
  import { statusText, statusTone, type TerminalStatus } from "../terminal/status";

  interface Props {
    status: TerminalStatus;
    kind: Launch["kind"];
    onStop: () => void;
    onRestart: () => void;
  }

  let { status, kind, onStop, onRestart }: Props = $props();
  const noun = $derived(kind === "script" ? "script" : "shell");
  const busy = $derived(status.kind === "starting" || status.kind === "stopping");
  const tone = $derived(statusTone(status));
</script>

<span
  class="status"
  class:ok={status.kind === "running"}
  class:bad={tone === "error"}
  title={statusText(status)}
>
  {statusText(status)}
</span>
<button
  class="icon"
  onclick={onRestart}
  disabled={busy}
  aria-label="Restart"
  title={kind === "script"
    ? `Stop and run the script again (${shortcutLabel("restart-tab")})`
    : `Stop and open a new shell (${shortcutLabel("restart-tab")})`}>↻</button
>
<button
  class="danger"
  onclick={onStop}
  disabled={status.kind !== "running"}
  title={`Stop the ${noun} and every process it started (${shortcutLabel("stop-tab")})`}
>
  <span class="square" aria-hidden="true"></span>Stop
</button>

<style>
  .status {
    max-width: 14rem;
    font-size: 0.8rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .status.ok {
    color: var(--accent);
  }

  .status.bad {
    color: var(--danger);
  }

  button {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.2rem 0.6rem;
    font-size: 0.8rem;
  }

  .icon {
    padding: 0.2rem 0.5rem;
    font-size: 0.95rem;
    line-height: 1.2;
  }

  .square {
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 1px;
    background: currentColor;
  }
</style>
