//! Several sessions at once, and the app-exit path: `stop_all` must leave no process of any
//! session behind.

mod common;

use std::time::Duration;

use runbook_process::SessionManager;

use common::{eventually, options, start, wait_for_output};

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::process::Command;

    use super::*;

    fn alive(pid: u32) -> bool {
        let out = Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .expect("ps");
        let stat = String::from_utf8_lossy(&out.stdout);
        let stat = stat.trim();
        !stat.is_empty() && !stat.starts_with('Z')
    }

    #[test]
    fn stop_all_leaves_no_process_of_any_session() {
        let manager = SessionManager::default();
        let tmp = tempfile::tempdir().unwrap();
        let mut pids = Vec::new();
        let mut receivers = Vec::new();

        for i in 0..3 {
            let dir = tmp.path().join(format!("s{i}"));
            fs::create_dir(&dir).unwrap();
            // Two children per session; one only dies by SIGKILL.
            let script = "sleep 300 & echo $! >> pids; \
                          sh -c 'trap \"\" TERM HUP; sleep 300' & echo $! >> pids; \
                          echo ready; wait";
            let (session, rx) = start(options("sh", &["-c", script], &dir));
            wait_for_output(&rx, "ready");
            pids.push(session.pid().unwrap());
            pids.extend(
                fs::read_to_string(dir.join("pids"))
                    .unwrap()
                    .lines()
                    .map(|l| l.trim().parse::<u32>().unwrap()),
            );
            manager.insert(session);
            receivers.push(rx);
        }
        assert_eq!(manager.len(), 3);
        assert!(pids.iter().all(|&p| alive(p)), "trees did not start");

        let outcomes = manager.stop_all(Duration::from_millis(500));
        assert_eq!(outcomes.len(), 3);
        assert!(manager.is_empty());
        assert!(
            eventually(Duration::from_secs(3), || pids.iter().all(|&p| !alive(p))),
            "survivors: {:?}",
            pids.iter().filter(|&&p| alive(p)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn ids_are_unique_and_removal_forgets_the_session() {
        let manager = SessionManager::default();
        let tmp = tempfile::tempdir().unwrap();
        let (a, _rx_a) = start(options("sh", &["-c", "sleep 300"], tmp.path()));
        let (b, _rx_b) = start(options("sh", &["-c", "sleep 300"], tmp.path()));
        let id_a = manager.insert(a);
        let id_b = manager.insert(b);
        assert_ne!(id_a, id_b);
        assert!(manager.get(id_a).is_some());
        assert!(manager.remove(id_a).is_some());
        assert!(manager.get(id_a).is_none());
        assert!(manager.get(id_b).is_some());
        manager.stop_all(Duration::from_millis(500));
    }
}

#[cfg(windows)]
mod windows {
    use std::process::Command;

    use super::*;

    fn alive(pid: u32) -> bool {
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .expect("tasklist");
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    }

    #[test]
    fn stop_all_leaves_no_process_of_any_session() {
        let manager = SessionManager::default();
        let tmp = tempfile::tempdir().unwrap();
        let mut pids = Vec::new();
        for _ in 0..2 {
            let (session, rx) = start(options(
                "cmd.exe",
                &["/d", "/c", "echo ready& ping -n 300 127.0.0.1 >nul"],
                tmp.path(),
            ));
            wait_for_output(&rx, "ready");
            pids.push(session.pid().unwrap());
            manager.insert(session);
        }
        manager.stop_all(Duration::from_secs(2));
        assert!(manager.is_empty());
        assert!(
            eventually(Duration::from_secs(5), || pids.iter().all(|&p| !alive(p))),
            "sessions survived stop_all"
        );
    }
}
