//! macOS: the process tree from libproc's `proc_listchildpids`.
//!
//! macOS assigns pids sequentially up to 99 999 before wrapping, so a pid is not reused
//! within the few seconds a stop takes; no start time is recorded.

use super::unix::Proc;

/// Upper bound of direct children read per process.
const MAX_CHILDREN: usize = 4096;

/// `leader` and every descendant, found by asking the kernel for each process's children.
pub fn tree(leader: libc::pid_t) -> Vec<Proc> {
    let mut tree = vec![Proc {
        pid: leader,
        start: None,
    }];
    let mut next = 0;
    while next < tree.len() {
        for pid in children(tree[next].pid) {
            if !tree.iter().any(|p| p.pid == pid) {
                tree.push(Proc { pid, start: None });
            }
        }
        next += 1;
    }
    tree
}

fn children(parent: libc::pid_t) -> Vec<libc::pid_t> {
    let mut pids = vec![0 as libc::pid_t; MAX_CHILDREN];
    let bytes = (pids.len() * size_of::<libc::pid_t>()) as libc::c_int;
    // SAFETY: the buffer is valid for `bytes` bytes; the call returns how many pids it wrote
    // (or -1 on error) and never writes past `bytes`.
    let count = unsafe { libc::proc_listchildpids(parent, pids.as_mut_ptr().cast(), bytes) };
    let count = usize::try_from(count).unwrap_or(0).min(pids.len());
    pids.truncate(count);
    pids.retain(|&pid| pid > 0);
    pids
}
