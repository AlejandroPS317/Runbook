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
  import { shortcutFor, shortcutLabel } from "../keyboard";
  import { statusText, statusTone, type TerminalStatus } from "../terminal/status";

  interface Props {
    packageDir: string;
    script: string;
    label: string;
    /** Only the active tab is visible; hidden tabs keep running and keep their output. */
    active: boolean;
    onStatus: (status: TerminalStatus) => void;
  }

  let { packageDir, script, label, active, onStatus }: Props = $props();

  let status = $state<TerminalStatus>({ kind: "starting" });
  let container: HTMLDivElement;

  // Set in onMount; the buttons and the active effect only run after mount.
  let restart: () => Promise<void> = async () => {};
  let stop: () => Promise<void> = async () => {};
  let show: () => void = () => {};

  /** Stops the script and its process tree (keyboard shortcut and ■ Stop). */
  export function stopScript(): Promise<void> {
    return stop();
  }

  /** Stops the script if needed and runs it again (keyboard shortcut and ↻ Restart). */
  export function restartScript(): Promise<void> {
    return status.kind === "starting" || status.kind === "stopping" ? Promise.resolve() : restart();
  }

  $effect(() => onStatus(status));
  $effect(() => {
    if (active) show();
  });

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
    // App shortcuts are not sent to the program; they bubble up to the window handler.
    terminal.attachCustomKeyEventHandler((event) => shortcutFor(event) === null);
    terminal.open(container);
    fit.fit();

    let sessionId: number | null = null;
    let disposed = false;
    // Incremented on every start; events from an older run are ignored.
    let run = 0;

    // Hidden tabs have no size: fitting them would shrink the PTY to a few columns.
    function refit() {
      if (container.offsetWidth === 0 || container.offsetHeight === 0) return;
      const before = { cols: terminal.cols, rows: terminal.rows };
      fit.fit();
      const after = { cols: terminal.cols, rows: terminal.rows };
      if (sessionId !== null && (before.cols !== after.cols || before.rows !== after.rows)) {
        void resizeTerminal(sessionId, after);
      }
    }

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

    show = () => {
      // Wait for the browser to lay out the now-visible panel before measuring it.
      requestAnimationFrame(() => {
        if (disposed) return;
        refit();
        terminal.focus();
      });
    };

    const input = terminal.onData((data) => {
      if (sessionId !== null) void writeTerminal(sessionId, data);
    });
    const observer = new ResizeObserver(refit);
    observer.observe(container);
    start();
    if (active) show();

    return () => {
      disposed = true;
      observer.disconnect();
      input.dispose();
      if (sessionId !== null) void closeTerminal(sessionId);
      terminal.dispose();
    };
  });

  const busy = $derived(status.kind === "starting" || status.kind === "stopping");
  const tone = $derived(statusTone(status));
</script>

<section class="panel">
  <header>
    <span class="label">{label}</span>
    <span class="status" class:ok={status.kind === "running"} class:bad={tone === "error"}>
      {statusText(status)}
    </span>
    <div class="actions">
      <button
        onclick={() => stop()}
        disabled={status.kind !== "running"}
        title={`Stop the script and every process it started (${shortcutLabel("stop-tab")})`}
      >
        ■ Stop
      </button>
      <button
        onclick={() => restart()}
        disabled={busy}
        title={`Stop and run the script again (${shortcutLabel("restart-tab")})`}
      >
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
