import type { Branch, GitStatus } from "./ipc/types";

/** The branch name, or the short commit in detached HEAD. */
export function headLabel(status: GitStatus): string {
  return status.branch ?? status.commit?.slice(0, 7) ?? "no commits";
}

export function filterBranches(branches: Branch[], query: string): Branch[] {
  const needle = query.trim().toLowerCase();
  return needle ? branches.filter((b) => b.name.toLowerCase().includes(needle)) : branches;
}

export function branchKey(branch: Branch): string {
  return `${branch.remote ?? ""}\0${branch.name}`;
}

// Where HEAD was last seen per project root. Module state, so it survives the status bar
// being re-created (the home screen replaces it while shown).
const seenHeads = new Map<string, string>();

function headKey(status: GitStatus): string {
  return `${status.branch ?? ""}\0${status.commit ?? ""}`;
}

/** Records where HEAD is now, after Runbook itself moved it and re-read the project. */
export function rememberHead(root: string, status: GitStatus): void {
  seenHeads.set(root, headKey(status));
}

/**
 * Records where HEAD is and reports whether it moved since it was last seen, for example
 * after a `git switch` in a terminal. Then the project's files may differ and its scripts
 * must be read again. The first sighting of a project is not a move.
 */
export function headMoved(root: string, status: GitStatus): boolean {
  const before = seenHeads.get(root);
  rememberHead(root, status);
  return before !== undefined && before !== headKey(status);
}
