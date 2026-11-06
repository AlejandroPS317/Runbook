//! Stopping a script together with every process it started.
//!
//! `turbo`, `pnpm` and `vite` start child processes; stopping only the direct child would
//! leave servers running. Each platform module terminates the whole tree:
//!
//! - Linux/macOS (`unix.rs`): SIGTERM to the script's process group and to every
//!   descendant found by walking parent links (which also catches descendants that left
//!   the group with `setsid`), a grace period, then SIGKILL to whatever is left.
//! - Windows (`windows.rs`): Ctrl+C through the console (handled in `Session::stop`), a
//!   grace period, then `taskkill /T /F`, which walks the tree by parent process id.

use std::time::Duration;

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

/// How long a stopped script gets to exit cleanly before it is killed.
pub const DEFAULT_GRACE: Duration = Duration::from_secs(3);

/// How long to wait for processes to disappear after the forced kill.
pub const KILL_WAIT: Duration = Duration::from_secs(2);

/// What happened when a session was stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
    /// The script had already exited; nothing was signalled.
    AlreadyExited,
    /// Every process exited within the grace period.
    Graceful,
    /// Some process ignored the graceful request and was killed.
    Forced,
    /// These process ids were still alive after the forced kill.
    Survivors(Vec<u32>),
}
