//! Process-tree termination on Linux and macOS.
//!
//! The script's direct child is a session and process-group leader (portable-pty runs
//! `setsid()`), so its pid is also its process-group id. Callers only stop a tree while the
//! leader is still running: then the pid cannot have been reused by another process.

use std::thread;
use std::time::{Duration, Instant};

use super::{KILL_WAIT, StopOutcome};

/// A process found in the tree. `start` (Linux only) identifies it across pid reuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Proc {
    pub pid: libc::pid_t,
    pub start: Option<u64>,
}

/// Liveness is re-checked at this interval during the grace period only; nothing polls
/// while scripts run.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// SIGTERM the tree rooted at `leader`, wait up to `grace`, then SIGKILL what is left.
pub fn terminate(leader: libc::pid_t, grace: Duration) -> StopOutcome {
    let mut targets = snapshot(leader);
    if !anything_alive(leader, &targets) {
        return StopOutcome::AlreadyExited;
    }
    signal_tree(leader, &targets, libc::SIGTERM);
    if wait_until_gone(leader, &targets, grace) {
        return StopOutcome::Graceful;
    }

    // Survivors may have started new children during the grace period.
    let survivors: Vec<Proc> = targets.iter().copied().filter(is_alive).collect();
    for proc in survivors {
        for found in snapshot(proc.pid) {
            if !targets.iter().any(|t| t.pid == found.pid) {
                targets.push(found);
            }
        }
    }
    signal_tree(leader, &targets, libc::SIGKILL);
    if wait_until_gone(leader, &targets, KILL_WAIT) {
        StopOutcome::Forced
    } else {
        StopOutcome::Survivors(
            targets
                .iter()
                .filter(|p| is_alive(p))
                .map(|p| p.pid as u32)
                .collect(),
        )
    }
}

/// SIGKILL the tree immediately, without waiting. Used when a running session is dropped
/// without being stopped first.
pub fn kill_now(leader: libc::pid_t) {
    let targets = snapshot(leader);
    signal_tree(leader, &targets, libc::SIGKILL);
}

/// The leader and all its current descendants.
fn snapshot(leader: libc::pid_t) -> Vec<Proc> {
    #[cfg(target_os = "linux")]
    let tree = super::linux::tree(leader);
    #[cfg(target_os = "macos")]
    let tree = super::macos::tree(leader);
    // Other Unix systems: no descendant discovery; the process-group signal still applies.
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let tree = vec![Proc {
        pid: leader,
        start: None,
    }];
    tree
}

fn signal_tree(leader: libc::pid_t, targets: &[Proc], signal: libc::c_int) {
    // SAFETY: kill(2) only takes integers. A negative pid addresses the process group whose
    // id is `leader`, i.e. the script's group.
    unsafe {
        libc::kill(-leader, signal);
    }
    for proc in targets.iter().filter(|p| is_alive(p)) {
        // SAFETY: as above; `is_alive` just confirmed this is still the same process.
        unsafe {
            libc::kill(proc.pid, signal);
        }
    }
}

fn anything_alive(leader: libc::pid_t, targets: &[Proc]) -> bool {
    group_exists(leader) || targets.iter().any(is_alive)
}

/// Whether any process still belongs to the script's process group.
fn group_exists(leader: libc::pid_t) -> bool {
    // SAFETY: signal 0 performs only the existence and permission check.
    let result = unsafe { libc::kill(-leader, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn is_alive(proc: &Proc) -> bool {
    #[cfg(target_os = "linux")]
    {
        super::linux::is_alive(proc)
    }
    #[cfg(not(target_os = "linux"))]
    {
        // SAFETY: signal 0 performs only the existence and permission check.
        let result = unsafe { libc::kill(proc.pid, 0) };
        result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
}

fn wait_until_gone(leader: libc::pid_t, targets: &[Proc], timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if !anything_alive(leader, targets) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(POLL_INTERVAL);
    }
}
