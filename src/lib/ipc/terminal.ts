import { Channel, invoke } from "@tauri-apps/api/core";
import type { ExitInfo, TerminalSize } from "./types";

export interface TerminalHandlers {
  /** Raw terminal output; may split UTF-8 sequences, which xterm.js handles. */
  onOutput(data: Uint8Array): void;
  /** Called once, after the last output. */
  onExit(info: ExitInfo): void;
}

/** Runs a script of the open project in a new PTY session and returns its id. */
export function runScript(
  packageDir: string,
  script: string,
  size: TerminalSize,
  handlers: TerminalHandlers,
): Promise<number> {
  // The backend sends output as raw bytes, which arrive as an ArrayBuffer.
  const onOutput = new Channel<ArrayBuffer>();
  onOutput.onmessage = (buffer) => handlers.onOutput(new Uint8Array(buffer));
  const onExit = new Channel<ExitInfo>();
  onExit.onmessage = (info) => handlers.onExit(info);
  return invoke<number>("run_script", {
    packageDir,
    script,
    cols: size.cols,
    rows: size.rows,
    onOutput,
    onExit,
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
