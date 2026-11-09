import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Project, RecentProject } from "./types";

/** Recently opened projects whose folder still exists, most recent first. */
export function recentProjects(): Promise<RecentProject[]> {
  return invoke<RecentProject[]>("recent_projects");
}

export function forgetRecentProject(path: string): Promise<void> {
  return invoke("forget_recent_project", { path });
}

/** Shows the native folder picker. Resolves to `null` when the user cancels. */
export async function pickFolder(): Promise<string | null> {
  const selection = await open({ directory: true, multiple: false });
  return typeof selection === "string" ? selection : null;
}

/** The folder passed on the command line (`runbook <folder>`); only returned once. */
export function takeInitialPath(): Promise<string | null> {
  return invoke<string | null>("take_initial_path");
}

/**
 * Discovers the project at `path` and adds it to the open projects. Opening one that is
 * already open returns the same `root` and keeps its running scripts.
 */
export function openProject(path: string): Promise<Project> {
  return invoke<Project>("open_project", { path });
}

/** Closes the project and stops every script it started, with its process tree. */
export function closeProject(root: string): Promise<void> {
  return invoke("close_project", { root });
}
