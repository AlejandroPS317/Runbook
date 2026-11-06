<script lang="ts">
  import { FitAddon } from "@xterm/addon-fit";
  import { Terminal } from "@xterm/xterm";
  import "@xterm/xterm/css/xterm.css";
  import { onMount } from "svelte";
  import {
    closeTerminal,
    resizeTerminal,
    runScript,
    stopTerminal,
    writeTerminal,
  } from "../ipc/terminal";
  import type { ExitInfo } from "../ipc/types";

  interface Props {
    packageDir: string;
    script: string;
    label: string;
  }

  let { packageDir, script, label }: Props = $props();

  type Status =
    | { kind: "starting" }
    | { kind: "running" }
    | { kind: "stopping" }
    | { kind: "exited"; info: ExitInfo }
    | { kind: "failed"; message: string };

  let status = $state<Status>({ kind: "starting" });
  let container: HTMLDivElement;

  // Set in onMount; the buttons only exist after mount.
  let restart: () => Promise<void> = async () => {};
  let stop: () => Promise<void> = async () => {};

  function statusText(current: Status): string {
    switch (current.kind) {
      case "starting":
        return "Starting…";
      case "running":
        return "Running";
      case "stopping":
        return "Stopping…";
      case "exited":
        return current.info.signal
          ? `Stopped (${current.info.signal})`
          : `Exited with code ${current.info.code}`;
      case "failed":
        return current.message;
      default: {
        const unreachable: never = current;
        return unreachable;
      }
    }
  }

  onMount(() => {
    const terminal = new Terminal({
      fontFamily: "ui-monospace, 'JetBrains Mono', 'DejaVu Sans Mono', monospace",
      fontSize: 13,
      scrollback: 10_000,
      cursorBlink: true,
      theme: { background: "#111113", foreground: "#e4e4e7" },
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(container);
    fit.fit();

    let sessionId: number | null = null;
    let disposed = false;
    // Incremented on every start; events from an older run are ignored.
    let run = 0;

    function start() {
      const current = ++run;
      status = { kind: "starting" };
      runScript(
        packageDir,
        script,
        { cols: terminal.cols, rows: terminal.rows },
        {
          onOutput: (data) => {
            if (current === run) terminal.write(data);
          },
          onExit: (info) => {
            if (current === run) status = { kind: "exited", info };
          },
        },
      ).then(
        (id) => {
          if (disposed || current !== run) {
            void closeTerminal(id);
            return;
          }
          sessionId = id;
          if (status.kind === "starting") status = { kind: "running" };
        },
        (err) => {
          if (current === run) status = { kind: "failed", message: String(err) };
        },
      );
    }

    stop = async () => {
      if (sessionId === null || status.kind !== "running") return;
      status = { kind: "stopping" };
      await stopTerminal(sessionId);
    };

    restart = async () => {
      const previous = sessionId;
      sessionId = null;
      run += 1;
      status = { kind: "stopping" };
      if (previous !== null) await closeTerminal(previous);
      if (disposed) return;
      terminal.reset();
      start();
    };

    const input = terminal.onData((data) => {
      if (sessionId !== null) void writeTerminal(sessionId, data);
    });

    // Refit when the panel changes size; only tell the PTY when the grid really changed.
    const observer = new ResizeObserver(() => {
      const before = { cols: terminal.cols, rows: terminal.rows };
      fit.fit();
      const after = { cols: terminal.cols, rows: terminal.rows };
      if (sessionId !== null && (before.cols !== after.cols || before.rows !== after.rows)) {
        void resizeTerminal(sessionId, after);
      }
    });
    observer.observe(container);
    start();

    return () => {
      disposed = true;
      observer.disconnect();
      input.dispose();
      if (sessionId !== null) void closeTerminal(sessionId);
      terminal.dispose();
    };
  });

  const busy = $derived(status.kind === "starting" || status.kind === "stopping");
</script>

<section class="panel">
  <header>
    <span class="label">{label}</span>
    <span
      class="status"
      class:ok={status.kind === "running"}
      class:bad={status.kind === "failed" || (status.kind === "exited" && status.info.code !== 0)}
    >
      {statusText(status)}
    </span>
    <div class="actions">
      <button onclick={() => stop()} disabled={status.kind !== "running"} title="Stop the script and every process it started">
        ■ Stop
      </button>
      <button onclick={() => restart()} disabled={busy} title="Stop and run the script again">
        ↻ Restart
      </button>
    </div>
  </header>
  <div class="terminal" bind:this={container}></div>
</section>

<style>
  .panel {
    display: grid;
    grid-template-rows: auto 1fr;
    height: 100%;
    min-height: 0;
    background: #111113;
  }

  header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.3rem 0.5rem 0.3rem 0.75rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    font-size: 0.85rem;
  }

  .label {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .status {
    color: var(--text-muted);
    white-space: nowrap;
  }

  .status.ok {
    color: var(--accent);
  }

  .status.bad {
    color: var(--danger);
  }

  .actions {
    display: flex;
    gap: 0.35rem;
  }

  .actions button {
    padding: 0.2rem 0.6rem;
    font-size: 0.8rem;
  }

  .terminal {
    min-height: 0;
    padding: 0.25rem 0 0 0.5rem;
  }
</style>
