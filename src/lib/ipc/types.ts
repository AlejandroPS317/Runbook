// Mirrors the DTOs in src-tauri/src/project.rs, src-tauri/src/terminal.rs,
// src-tauri/src/shell.rs and src-tauri/src/git.rs.

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

/** Mirrors `GitStatusDto` in src-tauri/src/git.rs. */
export interface GitStatus {
  /** `null` in detached HEAD. */
  branch: string | null;
  /** Commit `HEAD` points to; `null` before the first commit. */
  commit: string | null;
  upstream: string | null;
  /** Commits ahead of and behind the upstream, as of the last fetch. */
  ahead: number;
  behind: number;
  /** Modified, staged, unmerged and untracked files. */
  changes: number;
}

/** Mirrors `BranchDto` in src-tauri/src/git.rs. */
export interface Branch {
  name: string;
  /** Set for remote branches that have no local branch of that name. */
  remote: string | null;
  current: boolean;
}

/** Mirrors `GitChangeDto` in src-tauri/src/git.rs: a switch or pull moved the branch. */
export interface GitChange {
  status: GitStatus;
  /** The project read again; `null` when that failed (see `discoveryError`). */
  project: Project | null;
  discoveryError: string | null;
}

/** Mirrors `InstallEvent` in src-tauri/src/updates.rs. */
export type InstallEvent = { kind: "progress"; percent: number } | { kind: "installing" };
