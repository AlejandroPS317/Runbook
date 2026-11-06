// Mirrors the DTOs in src-tauri/src/project.rs and src-tauri/src/terminal.rs.

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

export interface ExitInfo {
  code: number;
  signal: string | null;
}

export interface TerminalSize {
  cols: number;
  rows: number;
}
