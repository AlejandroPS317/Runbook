//! Windows: forced termination of a process tree.
//!
//! The graceful step (Ctrl+C written to the pseudoconsole, which delivers CTRL_C_EVENT to
//! every process attached to it) lives in `Session::stop`, because it needs the session's
//! input pipe. Processes that detach from the console and outlive their parent are not
//! found by `taskkill /T`; a Job Object would cover them (planned, see process-control.md).

use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

/// CREATE_NO_WINDOW: run taskkill without flashing a console window.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `taskkill /T /F`: terminates `pid` and every process whose parent chain leads to it.
pub fn kill_tree(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}
