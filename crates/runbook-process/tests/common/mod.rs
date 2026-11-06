//! Helpers shared by the integration tests.

#![allow(dead_code)] // Each test file uses a different subset.

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Weak};
use std::thread;
use std::time::{Duration, Instant};

use runbook_process::{ExitInfo, Session, SessionEvent, SpawnOptions, TerminalSize};

pub const DEADLINE: Duration = Duration::from_secs(10);

pub fn options(program: &str, args: &[&str], cwd: &Path) -> SpawnOptions {
    SpawnOptions {
        program: program.into(),
        args: args.iter().map(|a| a.to_string()).collect(),
        cwd: cwd.to_path_buf(),
        env: std::env::vars().collect(),
        size: TerminalSize { rows: 24, cols: 80 },
    }
}

/// Spawns a session and returns its events, acting like a terminal emulator: cursor
/// position queries (`ESC[6n`) are answered as xterm.js would. Windows needs it:
/// portable-pty creates the ConPTY with PSEUDOCONSOLE_INHERIT_CURSOR, so conhost waits for
/// the answer before the child starts. The responder holds only a weak reference, so
/// dropping the returned `Arc` really drops the session.
pub fn start(options: SpawnOptions) -> (Arc<Session>, Receiver<SessionEvent>) {
    let (raw_tx, raw_rx) = mpsc::channel();
    let session = Arc::new(
        Session::spawn(options, move |event| {
            let _ = raw_tx.send(event);
        })
        .expect("spawn"),
    );
    let (tx, rx) = mpsc::channel();
    let responder: Weak<Session> = Arc::downgrade(&session);
    thread::spawn(move || {
        for event in raw_rx {
            if let SessionEvent::Output(bytes) = &event
                && bytes.windows(4).any(|w| w == b"\x1b[6n")
                && let Some(session) = responder.upgrade()
            {
                let _ = session.write(b"\x1b[1;1R");
            }
            if tx.send(event).is_err() {
                break;
            }
        }
    });
    (session, rx)
}

/// Collects output until the exit event, failing the test after the deadline. Also checks
/// that nothing follows the exit event.
pub fn run_to_exit(rx: &Receiver<SessionEvent>) -> (String, ExitInfo) {
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
            SessionEvent::Output(bytes) => output.extend(bytes),
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

/// Waits until the accumulated output contains `needle`.
pub fn wait_for_output(rx: &Receiver<SessionEvent>, needle: &str) -> String {
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

/// Polls `condition` until it holds or `timeout` passes (tests only).
pub fn eventually(timeout: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if condition() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(50));
    }
}
