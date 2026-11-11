import { invoke } from "@tauri-apps/api/core";
import type { Branch, GitChange, GitStatus } from "./types";

/** `null` when the project root is not the top level of a Git repository, or without `git`. */
export function gitStatus(root: string): Promise<GitStatus | null> {
  return invoke<GitStatus | null>("git_status", { root });
}

/** Local branches (current first), then remote branches without a local copy. */
export function gitBranches(root: string): Promise<Branch[]> {
  return invoke<Branch[]>("git_branches", { root });
}

/** Switches branch; a remote branch gets a local tracking branch. Rejects with Git's message. */
export function gitSwitch(root: string, branch: Branch): Promise<GitChange> {
  return invoke<GitChange>("git_switch", { root, name: branch.name, remote: branch.remote });
}

/** Fetches every remote and prunes deleted remote branches. */
export function gitFetch(root: string): Promise<GitStatus> {
  return invoke<GitStatus>("git_fetch", { root });
}

/** Fast-forwards the current branch; rejects when it diverged from its upstream. */
export function gitPull(root: string): Promise<GitChange> {
  return invoke<GitChange>("git_pull", { root });
}
