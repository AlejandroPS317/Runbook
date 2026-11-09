<script lang="ts">
  import { FitAddon } from "@xterm/addon-fit";
  import { Terminal } from "@xterm/xterm";
  import "@xterm/xterm/css/xterm.css";
  import { onMount } from "svelte";
  import {
    closeTerminal,
    openShell,
    resizeTerminal,
    runScript,
    stopTerminal,
    writeTerminal,
    type TerminalHandlers,
  } from "../ipc/terminal";
  import type { Project, ShellSession } from "../ipc/types";
  import { projectIndexFor, shortcutFor } from "../keyboard";
  import type { Launch } from "../terminal/launch";
  import type { TerminalStatus } from "../terminal/status";
  import { currentTerminalTheme, watchTerminalTheme } from "../terminal/theme";

  /**
   * How long a shell's output must stay quiet before its prompt counts as ready, for shells
   * that never turn on bracketed paste mode (see `watchPrompt`).
   */
  const PROMPT_QUIET_MS = 1000;

  interface Props {
    projectRoot: string;
    /** Fixed for the tab's lifetime; restart launches the same thing again. */
    launch: Launch;
    packageManager: Project["packageManager"];
    /**
     * Only the active tab of the selected project is visible; hidden tabs keep running and
     * keep their output.
     */
    active: boolean;
    onStatus: (status: TerminalStatus) => void;
  }

  let { projectRoot, launch, packageManager, active, onStatus }: Props = $props();
  const command = $derived(commandLine(launch));

  /** What the header shows the tab running; the PTY is spawned from argv, not from this. */
  function commandLine(target: Launch): string {
    switch (target.kind) {
      case "script":
        return `${packageManager} run ${target.script}`;
      case "shell":
        return target.prefillScript === null ? "shell" : `shell: ${packageManager} run ${target.prefillScript} …`;
      default: {
        const unreachable: never = target;
        return unreachable;
      }
    }
  }

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
      theme: currentTerminalTheme(),
    });
    const unwatchTheme = watchTerminalTheme((theme) => {
      terminal.options.theme = theme;
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    // App shortcuts are not sent to the program; they bubble up to the window handler.
    terminal.attachCustomKeyEventHandler(
      (event) => shortcutFor(event) === null && projectIndexFor(event) === null,
    );
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

    // A shell tab may come with a command line to type at its first prompt. Typing it
    // earlier would make the kernel echo it and the line editor draw it a second time.
    let prefill: string | null = null;
    let promptReady = false;
    let promptTimer: ReturnType<typeof setTimeout> | undefined;

    function typePrefill() {
      if (!promptReady || prefill === null || sessionId === null) return;
      void writeTerminal(sessionId, prefill);
      prefill = null;
    }

    function markPromptReady() {
      clearTimeout(promptTimer);
      promptReady = true;
      typePrefill();
    }

    // Runs after xterm.js has parsed each output chunk. Line editors (bash 5.1+, zsh, fish)
    // turn on bracketed paste mode right when they start reading the prompt. Shells without
    // one (sh, dash) never do; for them the prompt counts as ready once output has been
    // quiet for PROMPT_QUIET_MS, which is inherently a timed guess.
    function watchPrompt() {
      if (launch.kind !== "shell" || promptReady) return;
      if (terminal.modes.bracketedPasteMode) {
        markPromptReady();
        return;
      }
      clearTimeout(promptTimer);
      promptTimer = setTimeout(markPromptReady, PROMPT_QUIET_MS);
    }

    function spawn(handlers: TerminalHandlers): Promise<ShellSession> {
      const size = { cols: terminal.cols, rows: terminal.rows };
      switch (launch.kind) {
        case "script":
          return runScript(projectRoot, launch.packageDir, launch.script, size, handlers).then(
            (id) => ({ id, prefill: null }),
          );
        case "shell":
          return openShell(projectRoot, launch.packageDir, launch.prefillScript, size, handlers);
        default: {
          const unreachable: never = launch;
          return unreachable;
        }
      }
    }

    function start() {
      const current = ++run;
      status = { kind: "starting" };
      prefill = null;
      promptReady = false;
      clearTimeout(promptTimer);
      spawn({
        onOutput: (data) => {
          if (current === run)
            terminal.write(data, () => {
              if (current === run) watchPrompt();
            });
        },
        onExit: (info) => {
          if (current === run) status = { kind: "exited", info };
        },
      }).then(
        (session) => {
          if (disposed || current !== run) {
            void closeTerminal(session.id);
            return;
          }
          sessionId = session.id;
          prefill = session.prefill;
          typePrefill();
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
      clearTimeout(promptTimer);
      unwatchTheme();
      observer.disconnect();
      input.dispose();
      if (sessionId !== null) void closeTerminal(sessionId);
      terminal.dispose();
    };
  });

</script>

<section class="panel">
  <header>
    <span class="prompt" aria-hidden="true">➜</span>
    <span class="command" title={command}>{command}</span>
    <span class="dir" title="Working folder">{launch.packageDir || "."}</span>
  </header>
  <div class="terminal" bind:this={container}></div>
</section>

<style>
  /* `--bg` matches the xterm.js background (see terminal/theme.ts). */
  .panel {
    display: grid;
    grid-template-rows: auto 1fr;
    height: 100%;
    min-height: 0;
    background: var(--bg);
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin: 0.6rem 0.75rem 0.4rem;
    padding: 0.45rem 0.75rem;
    font-family: var(--mono);
    font-size: 0.8rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }

  .prompt {
    color: var(--accent);
    font-weight: 700;
  }

  .command {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dir {
    flex: 0 1 auto;
    min-width: 0;
    color: var(--text-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .terminal {
    min-height: 0;
    padding: 0 0.25rem 0.25rem 0.75rem;
  }
</style>
