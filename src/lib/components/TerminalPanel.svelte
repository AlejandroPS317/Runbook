<script lang="ts">
  import { FitAddon } from "@xterm/addon-fit";
  import { Terminal } from "@xterm/xterm";
  import "@xterm/xterm/css/xterm.css";
  import { onMount } from "svelte";
  import { closeTerminal, resizeTerminal, runScript, writeTerminal } from "../ipc/terminal";
  import type { ExitInfo } from "../ipc/types";

  interface Props {
    packageDir: string;
    script: string;
    label: string;
  }

  let { packageDir, script, label }: Props = $props();

  type Status = { kind: "starting" } | { kind: "running" } | { kind: "exited"; info: ExitInfo } | { kind: "failed"; message: string };

  let status = $state<Status>({ kind: "starting" });
  let container: HTMLDivElement;

  function statusText(current: Status): string {
    switch (current.kind) {
      case "starting":
        return "Starting…";
      case "running":
        return "Running";
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

    runScript(
      packageDir,
      script,
      { cols: terminal.cols, rows: terminal.rows },
      {
        onOutput: (data) => terminal.write(data),
        onExit: (info) => (status = { kind: "exited", info }),
      },
    ).then(
      (id) => {
        if (disposed) {
          void closeTerminal(id);
          return;
        }
        sessionId = id;
        if (status.kind === "starting") status = { kind: "running" };
      },
      (err) => (status = { kind: "failed", message: String(err) }),
    );

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

    return () => {
      disposed = true;
      observer.disconnect();
      input.dispose();
      if (sessionId !== null) void closeTerminal(sessionId);
      terminal.dispose();
    };
  });
</script>

<section class="panel">
  <header>
    <span class="label">{label}</span>
    <span class="status" class:ok={status.kind === "running"} class:bad={status.kind === "failed" || (status.kind === "exited" && status.info.code !== 0)}>
      {statusText(status)}
    </span>
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
    justify-content: space-between;
    gap: 1rem;
    padding: 0.4rem 0.75rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    font-size: 0.85rem;
  }

  .label {
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

  .terminal {
    min-height: 0;
    padding: 0.25rem 0 0 0.5rem;
  }
</style>
