//! Shell tabs: the user's interactive shell runs in a PTY in the requested folder, runs what
//! is typed at its prompt, and stops like a script even though it ignores SIGTERM.

mod common;

#[cfg(unix)]
mod unix {
    use std::time::{Duration, Instant};

    use runbook_process::{Environment, KILL_WAIT, StopOutcome, user_shell};

    use crate::common::{options, run_to_exit, start, wait_for_output};

    #[test]
    fn interactive_shell_starts_in_the_folder_and_runs_typed_commands() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Environment::from([("SHELL".to_owned(), "/bin/sh".to_owned())]);
        let shell = user_shell(&env);
        let args: Vec<&str> = shell.args.iter().map(String::as_str).collect();
        let (session, rx) = start(options(&shell.program, &args, tmp.path()));

        // The echoed input shows `$(pwd -P)` literally; only the command's output has the
        // expanded folder. macOS temp folders are behind a symlink, hence the canonical path.
        session.write(b"echo \"cwd=$(pwd -P)\"\n").unwrap();
        let cwd = tmp.path().canonicalize().unwrap();
        wait_for_output(&rx, &format!("cwd={}", cwd.display()));

        session.write(b"exit 0\n").unwrap();
        let (_, exit) = run_to_exit(&rx);
        assert_eq!(exit.code, 0);
    }

    #[test]
    fn stop_ends_an_interactive_bash_that_ignores_sigterm() {
        let tmp = tempfile::tempdir().unwrap();
        let (session, rx) = start(options(
            "bash",
            &["--norc", "--noprofile", "-i"],
            tmp.path(),
        ));
        session.write(b"echo re\"\"ady\n").unwrap();
        wait_for_output(&rx, "ready");

        let grace = Duration::from_millis(500);
        let started = Instant::now();
        let outcome = session.stop(grace);
        assert_eq!(outcome, StopOutcome::Forced);
        assert!(started.elapsed() < grace + KILL_WAIT + Duration::from_secs(1));
        run_to_exit(&rx);
        assert!(session.has_exited());
    }
}
