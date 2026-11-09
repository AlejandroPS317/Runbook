//! Terminal sessions: run a script of an open project in a PTY and stream its output.
//! Mirrors `src/lib/ipc/terminal.ts`.

use std::sync::Arc;

use runbook_process::{
    DEFAULT_GRACE, Session, SessionEvent, SessionId, SpawnOptions, TerminalSize,
};
use serde::Serialize;
use tauri::State;
use tauri::ipc::{Channel, InvokeResponseBody};

use crate::error::CommandError;
use crate::fonts;
use crate::state::AppState;

#[derive(Serialize, Clone)]
pub struct ExitDto {
    code: u32,
    signal: Option<String>,
}

/// Runs `script` from the package at `package_dir` (relative to `project_root`) of an open
/// project.
///
/// Only scripts that exist in that project can be run; the command line is always
/// `<package manager> run <script>`, built from argv and never through a shell string.
/// Output arrives on `on_output` as raw bytes (an `ArrayBuffer` in JS); `on_exit` fires once.
#[allow(clippy::too_many_arguments)] // Tauri commands take their IPC arguments one by one.
#[tauri::command]
pub async fn run_script(
    project_root: String,
    package_dir: String,
    script: String,
    cols: u16,
    rows: u16,
    on_output: Channel<InvokeResponseBody>,
    on_exit: Channel<ExitDto>,
    state: State<'_, AppState>,
) -> Result<SessionId, CommandError> {
    let target = state
        .projects
        .resolve(&project_root, &package_dir, &script)?;

    let options = SpawnOptions {
        program: target.package_manager.command().to_owned(),
        args: vec!["run".to_owned(), script],
        cwd: target.cwd,
        env: script_environment(&state),
        size: TerminalSize { rows, cols },
    };
    let session = Session::spawn(options, move |event| match event {
        // A send fails only when the webview is gone; there is nobody left to tell.
        SessionEvent::Output(bytes) => {
            let _ = on_output.send(InvokeResponseBody::Raw(bytes));
        }
        SessionEvent::Exited(info) => {
            let _ = on_exit.send(ExitDto {
                code: info.code,
                signal: info.signal,
            });
        }
    })?;

    let id = state.sessions.insert(Arc::new(session));
    // The project may have been closed while the script was spawning; its sessions were
    // already stopped, so this one must not outlive it.
    if !state.projects.attach(&project_root, id) {
        if let Some(session) = state.sessions.remove(id) {
            stop_off_thread(session).await;
        }
        return Err(CommandError::ProjectNotOpen(project_root));
    }
    Ok(id)
}

/// Sends keyboard input typed in the terminal.
#[tauri::command]
pub async fn write_terminal(
    id: SessionId,
    data: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    Ok(session(&state, id)?.write(data.as_bytes())?)
}

/// Propagates a terminal resize to the PTY.
#[tauri::command]
pub async fn resize_terminal(
    id: SessionId,
    cols: u16,
    rows: u16,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    Ok(session(&state, id)?.resize(TerminalSize { rows, cols })?)
}

/// Stops the script and its whole process tree; the session stays so its output remains
/// visible. Resolves once everything has exited (at most the grace period plus a short
/// kill wait). The exit event is delivered on the session's `on_exit` channel as usual.
#[tauri::command]
pub async fn stop_terminal(id: SessionId, state: State<'_, AppState>) -> Result<(), CommandError> {
    let session = session(&state, id)?;
    stop_off_thread(session).await;
    Ok(())
}

/// Stops the script (if still running) and forgets the session.
#[tauri::command]
pub async fn close_terminal(id: SessionId, state: State<'_, AppState>) -> Result<(), CommandError> {
    state.projects.detach(id);
    if let Some(session) = state.sessions.remove(id) {
        stop_off_thread(session).await;
    }
    Ok(())
}

/// The login-shell environment, without Runbook's own font workaround (ADR 0006).
fn script_environment(state: &AppState) -> runbook_process::Environment {
    let mut env = state.environment().clone();
    fonts::scrub(&mut env);
    env
}

/// Clones the session out of the manager so no lock is held while it is used.
fn session(state: &AppState, id: SessionId) -> Result<Arc<Session>, CommandError> {
    state
        .sessions
        .get(id)
        .ok_or(CommandError::SessionNotFound(id))
}

/// `Session::stop` blocks for up to the grace period, so it runs on the blocking pool.
async fn stop_off_thread(session: Arc<Session>) {
    let _ = tauri::async_runtime::spawn_blocking(move || session.stop(DEFAULT_GRACE)).await;
}
