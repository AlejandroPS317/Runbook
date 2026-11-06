//! Linux: the process tree from `/proc`.
//!
//! `/proc/<pid>/stat` gives the parent pid, the state (`Z` for zombies, which are already
//! dead and only wait to be reaped) and the start time, which tells a process apart from a
//! later one that reused its pid.

use std::collections::HashMap;
use std::fs;

use super::unix::Proc;

struct Stat {
    ppid: libc::pid_t,
    state: char,
    start: u64,
}

/// `leader` and every descendant, found by following parent links.
pub fn tree(leader: libc::pid_t) -> Vec<Proc> {
    let mut children: HashMap<libc::pid_t, Vec<Proc>> = HashMap::new();
    let mut leader_start = None;
    for (pid, stat) in all_processes() {
        if pid == leader {
            leader_start = Some(stat.start);
        }
        children.entry(stat.ppid).or_default().push(Proc {
            pid,
            start: Some(stat.start),
        });
    }

    let mut tree = vec![Proc {
        pid: leader,
        start: leader_start,
    }];
    let mut next = 0;
    while next < tree.len() {
        if let Some(kids) = children.get(&tree[next].pid) {
            tree.extend(kids.iter().copied());
        }
        next += 1;
    }
    tree
}

/// Running and the same process that was recorded (not a zombie, pid not reused).
pub fn is_alive(proc: &Proc) -> bool {
    match read_stat(proc.pid) {
        Some(stat) => stat.state != 'Z' && proc.start.is_none_or(|start| start == stat.start),
        None => false,
    }
}

fn all_processes() -> Vec<(libc::pid_t, Stat)> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| {
            entry
                .ok()?
                .file_name()
                .to_str()?
                .parse::<libc::pid_t>()
                .ok()
        })
        .filter_map(|pid| Some((pid, read_stat(pid)?)))
        .collect()
}

fn read_stat(pid: libc::pid_t) -> Option<Stat> {
    let text = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parse_stat(&text)
}

/// Parses `pid (comm) state ppid pgrp session tty_nr tpgid flags ... starttime ...`. The
/// command name may contain spaces and parentheses, so fields are counted after the last
/// `)`: state is field 3 of the line, ppid 4, starttime 22.
fn parse_stat(text: &str) -> Option<Stat> {
    let rest = &text[text.rfind(')')? + 1..];
    let fields: Vec<&str> = rest.split_whitespace().collect();
    Some(Stat {
        state: fields.first()?.chars().next()?,
        ppid: fields.get(1)?.parse().ok()?,
        start: fields.get(19)?.parse().ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stat_with_spaces_and_parens_in_the_name() {
        let line = "4242 (node (dev) x) S 4200 4242 4242 34816 4242 4194560 1 0 0 0 5 2 0 0 20 0 11 0 987654 1000 100 18446744073709551615";
        let stat = parse_stat(line).unwrap();
        assert_eq!(stat.state, 'S');
        assert_eq!(stat.ppid, 4200);
        assert_eq!(stat.start, 987654);
    }

    #[test]
    fn this_process_is_alive_and_part_of_its_own_tree() {
        let me = std::process::id() as libc::pid_t;
        let tree = tree(me);
        assert_eq!(tree[0].pid, me);
        assert!(is_alive(&tree[0]));
    }

    #[test]
    fn a_reused_pid_is_not_the_same_process() {
        let me = std::process::id() as libc::pid_t;
        let mut proc = tree(me)[0];
        proc.start = proc.start.map(|s| s + 1);
        assert!(!is_alive(&proc));
    }
}
