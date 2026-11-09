// Mirrors the DTOs in src-tauri/src/project.rs, src-tauri/src/terminal.rs and
// src-tauri/src/shell.rs.

export type PackageRole = "root" | "member" | "unlisted";

export interface Script {
  name: string;
  command: string;
}

export interface Package {
  /** Folder relative to the project root, `/`-separated; empty for the root package. */
  relativeDir: string;
  name: string | null;
  role: PackageRole;
  scripts: Script[];
}

export interface Problem {
  path: string;
  message: string;
}

export interface Project {
  root: string;
  name: string;
  packageManager: "pnpm" | "npm";
  packages: Package[];
  problems: Problem[];
}

/** Mirrors `RecentProject` in src-tauri/src/recent.rs. */
export interface RecentProject {
  path: string;
  name: string;
  /** Seconds since the Unix epoch. */
  openedAt: number;
}

export interface ExitInfo {
  code: number;
  signal: string | null;
}

export interface TerminalSize {
  cols: number;
  rows: number;
}

/** Mirrors `ShellDto` in src-tauri/src/shell.rs. */
export interface ShellSession {
  id: number;
  /** Command line to type at the first prompt, without a newline; `null` for a bare shell. */
  prefill: string | null;
}

/** Mirrors `UpdateInfo` in src-tauri/src/updates.rs. */
export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes: string | null;
}

/** Mirrors `InstallEvent` in src-tauri/src/updates.rs. */
export type InstallEvent = { kind: "progress"; percent: number } | { kind: "installing" };
