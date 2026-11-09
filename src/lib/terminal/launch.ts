/** What a terminal tab runs: a script, or the user's shell in a package folder. */
export type Launch =
  | { kind: "script"; packageDir: string; script: string }
  /** `prefillScript`: a script whose command line is typed at the prompt, for adding arguments. */
  | { kind: "shell"; packageDir: string; prefillScript: string | null };

/** Tab label, e.g. "web › dev", "web › shell" or "web › shell: db:migrate". */
export function launchLabel(owner: string, launch: Launch): string {
  switch (launch.kind) {
    case "script":
      return `${owner} › ${launch.script}`;
    case "shell":
      return launch.prefillScript === null
        ? `${owner} › shell`
        : `${owner} › shell: ${launch.prefillScript}`;
    default: {
      const unreachable: never = launch;
      return unreachable;
    }
  }
}
