# Process control and PTY rules

Reference for any code that spawns, signals or kills processes, or reads/writes a PTY.
This is the most fragile part of Runbook: `turbo`, `pnpm` and `vite` all create child
processes, and stopping a script must stop the whole tree on every platform.

## Requirements

- **Stop kills the whole tree**: the script's process and every descendant.
- **Restart** = stop (wait until the tree is gone) + spawn with the same command and cwd.
- **App exit leaves no orphans**: closing the window or quitting stops every session
  before the process exits.
- **No busy loops**: wait on process exit, PTY reads and channels. The only timer is the
  grace period between graceful and forced termination.

## Spawning

Implemented in `crates/runbook-process` (M3):

- Spawn through `portable-pty` (`native_pty_system()`), which uses a Unix PTY on Linux and
  macOS and ConPTY on Windows (`session.rs`).
- Build the command from argv: package manager binary, `run`, script name, extra args.
  `src-tauri/src/terminal.rs` only accepts scripts that exist in the open project.
- Set `cwd` to the directory of the script's `package.json`.
- Use the environment of the user's login shell (`env.rs`): Linux/macOS run
  `$SHELL -l -i -c` once and read `env -0` between markers, with a 10 s deadline and a
  fallback to the app's own environment; Windows uses the app's environment.
- Read the PTY on a dedicated blocking thread; a forwarder thread coalesces queued output
  (up to 64 KiB) and sends it to the frontend as raw bytes through a Tauri `Channel`.
  The exit event is always last: on Unix it waits for the reader to hit EOF; on Windows
  ConPTY keeps the pipe open, so it is sent as soon as the child exits.

Until M4, stopping a session uses `portable-pty`'s killer: SIGHUP to the direct child on
Unix, TerminateProcess on Windows. That is not a reliable tree kill.

## Stopping, per platform

### Linux and macOS

- `portable-pty` makes the child a session leader (`setsid`), so its process group id
  equals its pid. Assert this in a test rather than assuming it.
- Send `SIGTERM` to the group (`kill(-pgid, SIGTERM)`), wait for the grace period
  (default 3 s), then send `SIGKILL` to the group.
- Descendants that call `setsid`/`setpgid` leave the group. On Linux, also collect the
  descendant pids from `/proc` before signaling and signal them directly. On macOS, use
  `sysctl`/`proc_listchildpids` for the same purpose if tests show escapes.

### Windows

- Assign the child to a Job Object created with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`;
  closing or terminating the job kills the tree.
- Known race: the child can spawn grandchildren before it is assigned to the job.
  Document how the implementation handles it (ADR in M4).
- Fallback: `taskkill /T /F /PID <pid>`.

## Comments

Every OS-specific block states, for each OS, what it does and why. Example:

```rust
// Linux/macOS: signal the whole process group created by setsid so that
// children spawned by turbo/pnpm/vite receive SIGTERM too.
// Windows: handled by the Job Object in `windows.rs`; this path is not compiled.
#[cfg(unix)]
fn terminate_group(pgid: Pid) -> Result<(), StopError> { /* ... */ }
```

## Tests

- Build a real tree in the test: a script that starts grandchildren (for example
  `sh -c 'sleep 300 & sleep 300'` on Unix, an equivalent `cmd`/PowerShell chain on
  Windows), plus a grandchild that ignores `SIGTERM` to exercise the forced path.
- After stop, assert that every recorded pid is gone.
- Wait with a deadline (poll for exit with a timeout in tests); a fixed `sleep` is not a
  synchronization mechanism.
- Gate each test with the matching `cfg` and run it on that platform's CI runner. A
  platform's milestone is done only after these tests passed there.
