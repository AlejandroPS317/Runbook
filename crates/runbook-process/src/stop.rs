//! Stopping a session's script together with its whole process tree.

use std::time::Duration;

use crate::session::Session;
use crate::tree::StopOutcome;

impl Session {
    /// Stops the script and every process it started, waiting up to `grace` for a clean
    /// exit before killing what is left. Blocks until done (at most `grace` plus a short
    /// kill wait), so call it off the UI thread.
    ///
    /// Only acts while the direct child is running: once it has exited (and been reaped)
    /// its pid may be reused, so nothing is signalled and `AlreadyExited` is returned.
    pub fn stop(&self, grace: Duration) -> StopOutcome {
        if self.exit.is_set() {
            return StopOutcome::AlreadyExited;
        }
        let Some(pid) = self.pid else {
            self.kill_child();
            return StopOutcome::Forced;
        };
        self.stop_tree(pid, grace)
    }

    /// Linux/macOS: SIGTERM to the process group and every descendant, grace, SIGKILL.
    #[cfg(unix)]
    fn stop_tree(&self, pid: u32, grace: Duration) -> StopOutcome {
        crate::tree::unix::terminate(pid as libc::pid_t, grace)
    }

    /// Windows: Ctrl+C through the pseudoconsole reaches every process attached to it
    /// (CTRL_C_EVENT), then `taskkill /T /F` if the script has not exited after `grace`.
    /// Batch files such as `pnpm.cmd` answer Ctrl+C with "Terminate batch job (Y/N)?" and
    /// keep running, so for them the forced path is the normal one.
    #[cfg(windows)]
    fn stop_tree(&self, pid: u32, grace: Duration) -> StopOutcome {
        let _ = self.write(b"\x03");
        if self.exit.wait(grace) {
            return StopOutcome::Graceful;
        }
        crate::tree::windows::kill_tree(pid);
        if self.exit.wait(crate::tree::KILL_WAIT) {
            StopOutcome::Forced
        } else {
            self.kill_child();
            StopOutcome::Survivors(vec![pid])
        }
    }

    /// Immediate tree kill without a grace period; used by `Drop`.
    pub(crate) fn kill_tree_now(&self) {
        match self.pid {
            // Linux/macOS: SIGKILL to the process group and every descendant.
            #[cfg(unix)]
            Some(pid) => crate::tree::unix::kill_now(pid as libc::pid_t),
            // Windows: taskkill /T /F on the tree.
            #[cfg(windows)]
            Some(pid) => crate::tree::windows::kill_tree(pid),
            None => self.kill_child(),
        }
    }
}
