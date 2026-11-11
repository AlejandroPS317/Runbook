//! Running `git`: built from argv, never through a shell, and never interactive.

use std::io;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::error::GitError;
use crate::repository::Environment;

/// One `git` invocation.
pub(crate) struct Invocation<'a> {
    pub dir: &'a Path,
    pub env: &'a Environment,
    pub args: &'a [&'a str],
    /// Set on top of `env`.
    pub extra_env: &'a [(&'a str, &'a str)],
    /// `None` waits for as long as Git runs.
    pub timeout: Option<Duration>,
}

/// Runs Git and returns its standard output, or its message when it fails.
pub(crate) fn run(invocation: &Invocation) -> Result<String, GitError> {
    let child = command(invocation)
        .spawn()
        .map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => GitError::NotInstalled,
            _ => GitError::Spawn(err),
        })?;
    let output = match invocation.timeout {
        None => child.wait_with_output().map_err(GitError::Spawn)?,
        Some(timeout) => wait_with_deadline(child, timeout)?,
    };
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(GitError::Failed(failure_message(&output)))
    }
}

fn command(invocation: &Invocation) -> Command {
    let mut command = Command::new("git");
    command
        .args(invocation.args)
        .current_dir(invocation.dir)
        .env_clear()
        .envs(invocation.env)
        .envs(invocation.extra_env.iter().copied())
        // HTTPS credential prompts fail instead of waiting for input nobody can type.
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    detach(&mut command);
    command
}

/// Linux/macOS: start Git in a new session, without a controlling terminal, so `ssh` cannot
/// ask for a passphrase or a host-key confirmation on the terminal Runbook was launched
/// from (it fails instead), and so a timeout can kill Git and its helpers as one group.
/// Windows: no terminal to prompt on; only hide the console window a GUI app's child would
/// otherwise open. Credential helpers such as Git Credential Manager show their own window.
#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: the closure runs in the forked child before exec and only calls `setsid`,
    // which is async-signal-safe and touches no memory shared with the parent.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(windows)]
fn detach(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    /// CREATE_NO_WINDOW: run git without flashing a console window.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

/// Waits for Git on a helper thread and kills its process tree when `timeout` expires.
/// The helper thread owns the child until it is reaped, so its pid cannot be reused by
/// another process while it may still be signalled.
fn wait_with_deadline(child: Child, timeout: Duration) -> Result<Output, GitError> {
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => result.map_err(GitError::Spawn),
        Err(_) => {
            kill_tree(pid);
            Err(GitError::TimedOut(timeout))
        }
    }
}

/// Linux/macOS: Git leads its own process group (see `detach`), so `SIGKILL` to the group
/// also ends the `ssh` or credential helper it started.
/// Windows: `taskkill /T /F` ends Git and every process whose parent chain leads to it.
#[cfg(unix)]
fn kill_tree(pid: u32) {
    if let Ok(pid) = libc::pid_t::try_from(pid) {
        // SAFETY: plain syscall; a negative pid addresses the process group led by `pid`.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
fn kill_tree(pid: u32) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

/// Git's message without its `hint:` lines, which explain commands Runbook does not offer.
fn failure_message(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let text = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    let message = text
        .lines()
        .filter(|line| !line.starts_with("hint:"))
        .collect::<Vec<_>>()
        .join("\n");
    match message.trim() {
        "" => format!("git exited with {}", output.status),
        trimmed => trimmed.to_owned(),
    }
}

// Unix only: the fake exit statuses are built with `ExitStatusExt::from_raw`, whose
// meaning differs per OS. `failure_message` itself has no OS-specific code.
#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn output(code: i32, stdout: &str, stderr: &str) -> Output {
        use std::os::unix::process::ExitStatusExt;
        Output {
            status: std::process::ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn failure_message_drops_hints_and_falls_back_to_stdout() {
        let diverged = "hint: Diverging branches can't be fast-forwarded\nhint: git merge --no-ff\nfatal: Not possible to fast-forward, aborting.\n";
        assert_eq!(
            failure_message(&output(128, "", diverged)),
            "fatal: Not possible to fast-forward, aborting."
        );
        assert_eq!(failure_message(&output(1, "out\n", "  \n")), "out");
        assert!(failure_message(&output(1, "", "")).starts_with("git exited with"));
    }
}
