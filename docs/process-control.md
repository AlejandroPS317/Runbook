# Process control and PTY rules

Reference for any code that spawns, signals or kills processes, or reads/writes a PTY.
This is the most fragile part of Runbook: `turbo`, `pnpm` and `vite` all create child
processes, and stopping a script must stop the whole tree on every platform.

## Requirements

- **Stop kills the whole tree**: the script's process and every descendant.
- **Restart** = stop (wait until the tree is gone) + spawn with the same command and cwd.
- **App exit leaves no orphans**: closing the window or quitting stops every session
  before the process exits.
- **No busy loops**: wait on process exit, PTY reads and channels. The only timed part is
  a stop: the grace period between graceful and forced termination, during which liveness
  is re-checked every 20 ms. Nothing polls while scripts run.

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
  The exit event is always last and only sent once the reader hit end of stream, so no
  output is lost. Unix reaches EOF by itself; on Windows ConPTY keeps the pipe open, so the
  forwarder closes the pseudoconsole when the child exits, which flushes and closes it.

Windows: `portable-pty` creates the pseudoconsole with `PSEUDOCONSOLE_INHERIT_CURSOR`, so
conhost first sends a cursor-position query (`ESC[6n`) and the child does not start until
the terminal answers (`ESC[row;colR`). xterm.js answers it automatically; tests and any
headless use must answer it themselves (see `tests/common/mod.rs`).

## Stopping, per platform

Implemented in `crates/runbook-process` (M4): `Session::stop(grace)` in `stop.rs`, the
platform code in `tree/`. The default grace is 3 s (`DEFAULT_GRACE`). `stop` only acts
while the script's direct child is running: afterwards its pid may be reused, so nothing
is signalled (`AlreadyExited`). Dropping a session whose script still runs kills the tree
immediately (`kill_tree_now`). On app exit, `src-tauri` stops every session in parallel
and waits for all of them.

### Linux and macOS (`tree/unix.rs`)

- `portable-pty` makes the child a session leader (`setsid`), so its process-group id
  equals its pid (asserted in `tests/stop.rs`).
- Snapshot the tree: the leader plus every descendant by parent links. Linux reads
  `/proc/<pid>/stat` (`tree/linux.rs`) and records each start time, so a reused pid is
  never signalled and zombies count as gone. macOS uses `proc_listchildpids`
  (`tree/macos.rs`). The snapshot catches descendants that left the group with `setsid`.
- `SIGTERM` to the group and to each live process of the snapshot; wait up to the grace
  period; re-snapshot the survivors' children; `SIGKILL` to the group and everything
  left; wait up to 2 s (`KILL_WAIT`).
- When the session leader dies, the kernel also sends `SIGHUP` to the terminal's
  foreground group; tests that exercise the forced path ignore both `TERM` and `HUP`.

### Windows (`stop.rs`, `tree/windows.rs`)

See [ADR 0005](decisions/0005-windows-tree-termination.md).

- Write Ctrl+C (`0x03`) to the pseudoconsole: conhost delivers `CTRL_C_EVENT` to every
  process attached to it. Wait for the direct child's exit (event-based, no polling).
- After the grace period, `taskkill /T /F /PID <pid>` terminates the tree by parent pid.
- When the child exits, the forwarder closes the pseudoconsole, which also ends any
  remaining console-attached process.
- Batch files (`pnpm.cmd`) answer Ctrl+C with "Terminate batch job (Y/N)?" and keep
  running, so for them the forced path is the normal one.

## Comments

Every OS-specific block states, for each OS, what it does and why. Example:

```rust
// Linux/macOS: signal the whole process group created by setsid so that
// children spawned by turbo/pnpm/vite receive SIGTERM too.
// Windows: handled by Ctrl+C and taskkill in `stop.rs`; this path is not compiled.
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
