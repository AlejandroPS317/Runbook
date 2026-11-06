import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Project } from "./types";

/** Shows the native folder picker. Resolves to `null` when the user cancels. */
export async function pickFolder(): Promise<string | null> {
  const selection = await open({ directory: true, multiple: false });
  return typeof selection === "string" ? selection : null;
}

/** The folder passed on the command line (`runbook <folder>`); only returned once. */
export function takeInitialPath(): Promise<string | null> {
  return invoke<string | null>("take_initial_path");
}

/** Discovers the project at `path` and makes it the open project. */
export function openProject(path: string): Promise<Project> {
  return invoke<Project>("open_project", { path });
}
