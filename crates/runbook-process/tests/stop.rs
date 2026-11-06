//! Stopping a session terminates the script's whole process tree. Each test builds a real
//! tree, records the pids, stops (or drops) the session and checks that every pid is gone.

mod common;

use std::time::Duration;

use runbook_process::StopOutcome;

use common::{eventually, options, run_to_exit, start, wait_for_output};

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::time::Instant;

    use runbook_process::SpawnOptions;

    use super::*;

    /// A script with three grandchildren: a plain one, one that leaves the process group
    /// with setsid (so a process-group signal alone would miss it), and one that ignores
    /// SIGTERM and SIGHUP (so only SIGKILL stops it; SIGHUP too, because the kernel hangs up
    /// the terminal's foreground group when the session leader dies). `perl` provides
    /// setsid portably (Linux and macOS).
    /// The setsid process records its own pid only after setsid(), and "ready" is printed
    /// once all three pids are recorded, so the test never inspects a half-started tree.
    const TREE: &str = r#"
sleep 300 & echo $! >> pids
sh -c 'trap "" TERM HUP; sleep 300' & echo $! >> pids
perl -e 'use POSIX qw(setsid); setsid(); open(my $f, ">>", "pids"); print $f "$$\n"; close $f; sleep 300' &
while [ "$(wc -l < pids | tr -d ' ')" -lt 3 ]; do sleep 0.05; done
echo ready
wait
"#;

    fn sh(script: &str, cwd: &Path) -> SpawnOptions {
        options("sh", &["-c", script], cwd)
    }

    /// Alive and not a zombie (a zombie has exited and only waits to be reaped).
    fn alive(pid: u32) -> bool {
        let out = Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .expect("ps");
        let stat = String::from_utf8_lossy(&out.stdout);
        let stat = stat.trim();
        !stat.is_empty() && !stat.starts_with('Z')
    }

    fn process_group(pid: u32) -> u32 {
        let out = Command::new("ps")
            .args(["-o", "pgid=", "-p", &pid.to_string()])
            .output()
            .expect("ps");
        String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
    }

    fn recorded_pids(dir: &Path) -> Vec<u32> {
        fs::read_to_string(dir.join("pids"))
            .unwrap()
            .lines()
            .map(|line| line.trim().parse().unwrap())
            .collect()
    }

    fn all_gone(pids: &[u32]) -> bool {
        eventually(Duration::from_secs(3), || {
            pids.iter().all(|&pid| !alive(pid))
        })
    }

    #[test]
    fn stop_kills_escaped_and_sigterm_ignoring_descendants() {
        let tmp = tempfile::tempdir().unwrap();
        let (session, rx) = start(sh(TREE, tmp.path()));
        wait_for_output(&rx, "ready");
        let leader = session.pid().unwrap();
        let pids = recorded_pids(tmp.path());
        assert_eq!(pids.len(), 3);
        assert!(pids.iter().all(|&pid| alive(pid)), "tree did not start");
        assert_eq!(
            process_group(pids[0]),
            leader,
            "plain child shares the group"
        );
        assert_ne!(
            process_group(pids[2]),
            leader,
            "setsid child left the group"
        );

        let started = Instant::now();
        let outcome = session.stop(Duration::from_millis(500));
        assert_eq!(outcome, StopOutcome::Forced);
        assert!(started.elapsed() >= Duration::from_millis(500));

        let mut everything = pids.clone();
        everything.push(leader);
        assert!(
            all_gone(&everything),
            "survivors: {:?}",
            everything.iter().filter(|&&p| alive(p)).collect::<Vec<_>>()
        );
        run_to_exit(&rx);
    }

    #[test]
    fn stop_is_graceful_when_everything_honors_sigterm() {
        let tmp = tempfile::tempdir().unwrap();
        let (session, rx) = start(sh(
            "sleep 300 & echo $! >> pids; sleep 300 & echo $! >> pids; echo ready; wait",
            tmp.path(),
        ));
        wait_for_output(&rx, "ready");
        let pids = recorded_pids(tmp.path());

        let started = Instant::now();
        let outcome = session.stop(Duration::from_secs(3));
        assert_eq!(outcome, StopOutcome::Graceful);
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(all_gone(&pids));
        let (_, exit) = run_to_exit(&rx);
        // Linux reports "Terminated", macOS "Terminated: 15".
        assert!(
            exit.signal
                .as_deref()
                .is_some_and(|s| s.starts_with("Terminated")),
            "{exit:?}"
        );
    }

    #[test]
    fn stop_after_the_script_exited_signals_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let (session, rx) = start(sh("exit 0", tmp.path()));
        run_to_exit(&rx);
        assert!(session.has_exited());
        assert_eq!(
            session.stop(Duration::from_secs(1)),
            StopOutcome::AlreadyExited
        );
    }

    #[test]
    fn dropping_a_running_session_kills_its_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let (session, rx) = start(sh(TREE, tmp.path()));
        wait_for_output(&rx, "ready");
        let mut pids = recorded_pids(tmp.path());
        pids.push(session.pid().unwrap());

        drop(session);
        assert!(
            all_gone(&pids),
            "survivors: {:?}",
            pids.iter().filter(|&&p| alive(p)).collect::<Vec<_>>()
        );
    }
}

#[cfg(windows)]
mod windows {
    use std::fs;
    use std::process::Command;

    use super::*;

    /// A batch file (like `pnpm.cmd`) that starts a background PowerShell grandchild.
    const TREE: &str = "@echo off\r\n\
start \"\" /b powershell -NoProfile -Command \"$PID | Out-File -Encoding ascii pid.txt; Start-Sleep 300\"\r\n\
echo ready\r\n\
ping -n 300 127.0.0.1 >nul\r\n";

    fn alive(pid: u32) -> bool {
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .expect("tasklist");
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    }

    #[test]
    fn stop_kills_the_batch_file_and_its_grandchild() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("tree.cmd"), TREE).unwrap();
        let (session, rx) = start(options("cmd.exe", &["/d", "/c", "tree.cmd"], tmp.path()));
        wait_for_output(&rx, "ready");
        let pid_file = tmp.path().join("pid.txt");
        assert!(
            eventually(Duration::from_secs(20), || fs::read_to_string(&pid_file)
                .is_ok_and(|s| s.trim().parse::<u32>().is_ok())),
            "grandchild never wrote its pid"
        );
        let grandchild: u32 = fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let leader = session.pid().unwrap();
        assert!(alive(grandchild));

        let outcome = session.stop(Duration::from_secs(2));
        assert!(
            matches!(outcome, StopOutcome::Graceful | StopOutcome::Forced),
            "{outcome:?}"
        );
        assert!(
            eventually(Duration::from_secs(5), || !alive(grandchild)
                && !alive(leader)),
            "processes survived the stop"
        );
        run_to_exit(&rx);
    }
}
