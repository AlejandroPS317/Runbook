import type { ExitInfo } from "../ipc/types";

/** Lifecycle of one terminal tab's script. */
export type TerminalStatus =
  | { kind: "starting" }
  | { kind: "running" }
  | { kind: "stopping" }
  | { kind: "exited"; info: ExitInfo }
  | { kind: "failed"; message: string };

export function statusText(status: TerminalStatus): string {
  switch (status.kind) {
    case "starting":
      return "Starting…";
    case "running":
      return "Running";
    case "stopping":
      return "Stopping…";
    case "exited":
      return status.info.signal
        ? `Stopped (${status.info.signal})`
        : `Exited with code ${status.info.code}`;
    case "failed":
      return status.message;
    default: {
      const unreachable: never = status;
      return unreachable;
    }
  }
}

/** Visual tone of a status: running, finished cleanly, or something went wrong. */
export type StatusTone = "active" | "idle" | "error";

export function statusTone(status: TerminalStatus): StatusTone {
  switch (status.kind) {
    case "starting":
    case "running":
    case "stopping":
      return "active";
    case "exited":
      return status.info.code === 0 || status.info.signal !== null ? "idle" : "error";
    case "failed":
      return "error";
    default: {
      const unreachable: never = status;
      return unreachable;
    }
  }
}
