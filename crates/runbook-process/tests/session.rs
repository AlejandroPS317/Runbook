//! PTY sessions with real processes. Unix tests use `/bin/sh`; Windows uses `cmd.exe`.

mod common;

use runbook_process::{Session, SessionError};

use common::{options, run_to_exit, start};

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
    use std::path::Path;

    use runbook_process::{SpawnOptions, TerminalSize};

    use super::common::wait_for_output;
    use super::*;

    fn sh(script: &str, cwd: &Path) -> SpawnOptions {
        options("sh", &["-c", script], cwd)
    }

    #[test]
    fn streams_output_and_reports_exit_code() {
        let tmp = tempfile::tempdir().unwrap();
        let (_session, rx) = start(sh("printf hello; exit 3", tmp.path()));
        let (output, exit) = run_to_exit(&rx);
        assert!(output.contains("hello"), "{output:?}");
        assert_eq!(exit.code, 3);
    }

    #[test]
    fn output_printed_right_before_exit_arrives_before_the_exit_event() {
        let tmp = tempfile::tempdir().unwrap();
        let (_session, rx) = start(sh("printf start; sleep 0.2; printf end", tmp.path()));
        let (output, exit) = run_to_exit(&rx);
        assert!(output.contains("startend"), "{output:?}");
        assert_eq!(exit.code, 0);
    }

    #[test]
    fn runs_in_the_requested_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let (_session, rx) = start(sh("pwd -P", tmp.path()));
        let (output, _) = run_to_exit(&rx);
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
        let (_session, rx) = start(opts);
        let (output, _) = run_to_exit(&rx);
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
        let (_session, rx) = start(opts);
        let (output, _) = run_to_exit(&rx);
        assert!(output.contains("30 100"), "{output:?}");
    }

    #[test]
    fn resize_and_keyboard_input_reach_the_program() {
        let tmp = tempfile::tempdir().unwrap();
        let (session, rx) = start(sh(
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
        let (output, exit) = run_to_exit(&rx);
        assert!(output.contains("40 120"), "{output:?}");
        assert!(output.contains("got:abc"), "{output:?}");
        assert_eq!(exit.code, 0);
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    #[test]
    fn streams_output_and_reports_exit_code() {
        let tmp = tempfile::tempdir().unwrap();
        let (_session, rx) = start(options(
            "cmd.exe",
            &["/d", "/c", "echo hello& exit /b 3"],
            tmp.path(),
        ));
        let (output, exit) = run_to_exit(&rx);
        assert!(output.contains("hello"), "{output:?}");
        assert_eq!(exit.code, 3);
    }
}
