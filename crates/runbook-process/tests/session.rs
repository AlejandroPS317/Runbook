//! PTY sessions with real processes. Unix tests use `/bin/sh`; Windows uses `cmd.exe`.

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use runbook_process::{ExitInfo, Session, SessionError, SessionEvent, SpawnOptions, TerminalSize};

const DEADLINE: Duration = Duration::from_secs(10);

fn options(program: &str, args: &[&str], cwd: &Path) -> SpawnOptions {
    SpawnOptions {
        program: program.into(),
        args: args.iter().map(|a| a.to_string()).collect(),
        cwd: cwd.to_path_buf(),
        env: std::env::vars().collect(),
        size: TerminalSize { rows: 24, cols: 80 },
    }
}

fn start(options: SpawnOptions) -> (Session, Receiver<SessionEvent>) {
    let (tx, rx) = mpsc::channel();
    let session = Session::spawn(options, move |event| {
        let _ = tx.send(event);
    })
    .expect("spawn");
    (session, rx)
}

/// Collects output until the exit event, failing the test after the deadline.
///
/// Answers cursor-position queries like a real terminal (xterm.js does the same). Windows
/// needs it: portable-pty creates the ConPTY with PSEUDOCONSOLE_INHERIT_CURSOR, so conhost
/// sends `ESC[6n` and waits for `ESC[row;colR` before the child starts.
fn run_to_exit(session: &mut Session, rx: &Receiver<SessionEvent>) -> (String, ExitInfo) {
    let started = Instant::now();
    let mut output = Vec::new();
    loop {
        let remaining = DEADLINE
            .checked_sub(started.elapsed())
            .expect("process did not exit in time");
        match rx
            .recv_timeout(remaining)
            .expect("process did not exit in time")
        {
            SessionEvent::Output(bytes) => {
                if bytes.windows(4).any(|w| w == b"\x1b[6n") {
                    session.write(b"\x1b[1;1R").unwrap();
                }
                output.extend(bytes);
            }
            SessionEvent::Exited(info) => {
                assert!(
                    rx.recv_timeout(Duration::from_millis(200)).is_err(),
                    "no event may follow the exit event"
                );
                return (String::from_utf8_lossy(&output).into_owned(), info);
            }
        }
    }
}

#[test]
fn missing_program_is_a_spawn_error() {
    let tmp = tempfile::tempdir().unwrap();
    let result = Session::spawn(
        options("runbook-no-such-program-xyz", &[], tmp.path()),
        |_| {},
    );
    assert!(matches!(result, Err(SessionError::Spawn { .. })));
}

#[cfg(unix)]
mod unix {
    use super::*;

    fn sh(script: &str, cwd: &Path) -> SpawnOptions {
        options("sh", &["-c", script], cwd)
    }

    /// Waits until the accumulated output contains `needle`.
    fn wait_for_output(rx: &Receiver<SessionEvent>, needle: &str) -> String {
        let started = Instant::now();
        let mut output = Vec::new();
        while !String::from_utf8_lossy(&output).contains(needle) {
            let remaining = DEADLINE
                .checked_sub(started.elapsed())
                .unwrap_or_else(|| panic!("never saw {needle:?}"));
            match rx.recv_timeout(remaining) {
                Ok(SessionEvent::Output(bytes)) => output.extend(bytes),
                Ok(SessionEvent::Exited(_)) | Err(_) => panic!(
                    "never saw {needle:?}; output: {:?}",
                    String::from_utf8_lossy(&output)
                ),
            }
        }
        String::from_utf8_lossy(&output).into_owned()
    }

    #[test]
    fn streams_output_and_reports_exit_code() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut session, rx) = start(sh("printf hello; exit 3", tmp.path()));
        let (output, exit) = run_to_exit(&mut session, &rx);
        assert!(output.contains("hello"), "{output:?}");
        assert_eq!(exit.code, 3);
    }

    #[test]
    fn output_printed_right_before_exit_arrives_before_the_exit_event() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut session, rx) = start(sh("printf start; sleep 0.2; printf end", tmp.path()));
        let (output, exit) = run_to_exit(&mut session, &rx);
        assert!(output.contains("startend"), "{output:?}");
        assert_eq!(exit.code, 0);
    }

    #[test]
    fn runs_in_the_requested_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut session, rx) = start(sh("pwd -P", tmp.path()));
        let (output, _) = run_to_exit(&mut session, &rx);
        let expected = tmp.path().canonicalize().unwrap();
        assert!(
            output.contains(expected.to_str().unwrap()),
            "{output:?} should contain {expected:?}"
        );
    }

    #[test]
    fn uses_the_given_environment_and_a_color_terminal() {
        let tmp = tempfile::tempdir().unwrap();
        let mut opts = sh(r#"printf '%s|%s' "$RUNBOOK_TEST_VAR" "$TERM""#, tmp.path());
        opts.env
            .insert("RUNBOOK_TEST_VAR".into(), "from-runbook".into());
        let (mut session, rx) = start(opts);
        let (output, _) = run_to_exit(&mut session, &rx);
        assert!(output.contains("from-runbook|xterm-256color"), "{output:?}");
    }

    #[test]
    fn starts_with_the_requested_size() {
        let tmp = tempfile::tempdir().unwrap();
        let mut opts = sh("stty size", tmp.path());
        opts.size = TerminalSize {
            rows: 30,
            cols: 100,
        };
        let (mut session, rx) = start(opts);
        let (output, _) = run_to_exit(&mut session, &rx);
        assert!(output.contains("30 100"), "{output:?}");
    }

    #[test]
    fn resize_and_keyboard_input_reach_the_program() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut session, rx) = start(sh(
            r#"printf ready; read line; stty size; printf 'got:%s' "$line""#,
            tmp.path(),
        ));
        wait_for_output(&rx, "ready");
        session
            .resize(TerminalSize {
                rows: 40,
                cols: 120,
            })
            .unwrap();
        // The terminal sends Enter as carriage return; the PTY line discipline maps it.
        session.write(b"abc\r").unwrap();
        let (output, exit) = run_to_exit(&mut session, &rx);
        assert!(output.contains("40 120"), "{output:?}");
        assert!(output.contains("got:abc"), "{output:?}");
        assert_eq!(exit.code, 0);
    }

    #[test]
    fn kill_stops_a_long_running_script() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut session, rx) = start(sh("printf started; sleep 30", tmp.path()));
        wait_for_output(&rx, "started");
        let started = Instant::now();
        session.kill().unwrap();
        let (_, exit) = run_to_exit(&mut session, &rx);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_ne!(exit.code, 0, "{exit:?}");
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    #[test]
    fn streams_output_and_reports_exit_code() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut session, rx) = start(options(
            "cmd.exe",
            &["/d", "/c", "echo hello& exit /b 3"],
            tmp.path(),
        ));
        let (output, exit) = run_to_exit(&mut session, &rx);
        assert!(output.contains("hello"), "{output:?}");
        assert_eq!(exit.code, 3);
    }
}
