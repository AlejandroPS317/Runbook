# 0005 – Process-tree termination on Windows

- Status: Accepted
- Date: 2026-11-06

## Context

`docs/process-control.md` planned Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) for
Windows and asked for an ADR on the race between spawning and assigning the job:
`portable-pty` creates the process itself and does not expose `CREATE_SUSPENDED`, so the
script can start grandchildren before it is assigned. Linux is the priority platform
([ADR 0002](0002-linux-first.md)), and Windows behavior can only be checked in CI.

## Decision

For M4, Windows stops a session in two steps, without Job Objects:

1. Ctrl+C written to the pseudoconsole input, which conhost turns into `CTRL_C_EVENT` for
   every process attached to the console. Wait up to the grace period for the direct
   child's exit.
2. `taskkill /T /F /PID <pid>`, which terminates the tree by parent process id.

Closing the pseudoconsole after the child exits also ends any process still attached to
it. The Windows CI test runs a batch file that starts a PowerShell grandchild and checks
both are gone after `stop`.

## Alternatives considered

- **Job Objects now**: robust for processes that detach from the console, but needs the
  `windows-sys` dependency, unsafe handle management, and still has the assignment race
  unless portable-pty is changed or replaced.
- **Only `TerminateProcess` on the child**: leaves `node`/`vite` grandchildren running.

## Consequences

- Covers the common case: `pnpm.cmd`/`npm.cmd` → `node` → tool processes, all attached to
  the pseudoconsole.
- A process that detaches (`DETACHED_PROCESS`, `CREATE_NEW_CONSOLE`) and whose parent has
  already exited is not found by `taskkill /T`. Revisit with Job Objects if a real tool
  does this.
- Ctrl+C makes batch files prompt "Terminate batch job (Y/N)?", so Windows stops usually
  take the full grace period before the forced step.
