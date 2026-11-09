import { Channel, invoke } from "@tauri-apps/api/core";
import type { ExitInfo, ShellSession, TerminalSize } from "./types";

export interface TerminalHandlers {
  /** Raw terminal output; may split UTF-8 sequences, which xterm.js handles. */
  onOutput(data: Uint8Array): void;
  /** Called once, after the last output. */
  onExit(info: ExitInfo): void;
}

function channels(handlers: TerminalHandlers) {
  // The backend sends output as raw bytes, which arrive as an ArrayBuffer.
  const onOutput = new Channel<ArrayBuffer>();
  onOutput.onmessage = (buffer) => handlers.onOutput(new Uint8Array(buffer));
  const onExit = new Channel<ExitInfo>();
  onExit.onmessage = (info) => handlers.onExit(info);
  return { onOutput, onExit };
}

/** Runs a script of the open project at `projectRoot` in a new PTY session; returns its id. */
export function runScript(
  projectRoot: string,
  packageDir: string,
  script: string,
  size: TerminalSize,
  handlers: TerminalHandlers,
): Promise<number> {
  return invoke<number>("run_script", {
    projectRoot,
    packageDir,
    script,
    cols: size.cols,
    rows: size.rows,
    ...channels(handlers),
  });
}

/**
 * Opens the user's shell in a package folder of the open project at `projectRoot` (empty
 * `packageDir` for the project root). With `script`, the result carries that script's command
 * line for the caller to type at the prompt.
 */
export function openShell(
  projectRoot: string,
  packageDir: string,
  script: string | null,
  size: TerminalSize,
  handlers: TerminalHandlers,
): Promise<ShellSession> {
  return invoke<ShellSession>("open_shell", {
    projectRoot,
    packageDir,
    script,
    cols: size.cols,
    rows: size.rows,
    ...channels(handlers),
  });
}

export function writeTerminal(id: number, data: string): Promise<void> {
  return invoke("write_terminal", { id, data });
}

export function resizeTerminal(id: number, size: TerminalSize): Promise<void> {
  return invoke("resize_terminal", { id, cols: size.cols, rows: size.rows });
}

/**
 * Stops the script and its whole process tree, keeping the session's output. Resolves once
 * everything has exited; `onExit` fires as usual.
 */
export function stopTerminal(id: number): Promise<void> {
  return invoke("stop_terminal", { id });
}

/** Stops the script if it is still running and releases the session. */
export function closeTerminal(id: number): Promise<void> {
  return invoke("close_terminal", { id });
}
