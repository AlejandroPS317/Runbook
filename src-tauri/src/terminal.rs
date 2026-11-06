//! Terminal sessions: run a script of the open project in a PTY and stream its output.
//! Mirrors `src/lib/ipc/terminal.ts`.

use runbook_process::{Session, SessionEvent, SpawnOptions, TerminalSize};
use serde::Serialize;
use tauri::State;
use tauri::ipc::{Channel, InvokeResponseBody};

use crate::error::CommandError;
use crate::state::{AppState, SessionId, lock};

#[derive(Serialize, Clone)]
pub struct ExitDto {
    code: u32,
    signal: Option<String>,
}

/// Runs `script` from the package at `package_dir` (relative to the project root).
///
/// Only scripts that exist in the open project can be run; the command line is always
/// `<package manager> run <script>`, built from argv and never through a shell string.
/// Output arrives on `on_output` as raw bytes (an `ArrayBuffer` in JS); `on_exit` fires once.
#[tauri::command]
pub async fn run_script(
    package_dir: String,
    script: String,
    cols: u16,
    rows: u16,
    on_output: Channel<InvokeResponseBody>,
    on_exit: Channel<ExitDto>,
    state: State<'_, AppState>,
) -> Result<SessionId, CommandError> {
    let (cwd, manager) = {
        let project = lock(&state.project);
        let project = project.as_ref().ok_or(CommandError::NoProject)?;
        let package = project
            .packages
            .iter()
            .find(|p| p.relative_dir == package_dir)
            .ok_or_else(|| CommandError::PackageNotFound(package_dir.clone()))?;
        if !package.scripts.iter().any(|s| s.name == script) {
            return Err(CommandError::ScriptNotFound {
                package: package_dir,
                script,
            });
        }
        (package.dir.clone(), project.package_manager)
    };

    let options = SpawnOptions {
        program: manager.command().to_owned(),
        args: vec!["run".to_owned(), script],
        cwd,
        env: state.environment().clone(),
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

    let id = state.next_session_id();
    lock(&state.sessions).insert(id, session);
    Ok(id)
}

/// Sends keyboard input typed in the terminal.
#[tauri::command]
pub async fn write_terminal(
    id: SessionId,
    data: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let mut sessions = lock(&state.sessions);
    let session = sessions
        .get_mut(&id)
        .ok_or(CommandError::SessionNotFound(id))?;
    Ok(session.write(data.as_bytes())?)
}

/// Propagates a terminal resize to the PTY.
#[tauri::command]
pub async fn resize_terminal(
    id: SessionId,
    cols: u16,
    rows: u16,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let sessions = lock(&state.sessions);
    let session = sessions.get(&id).ok_or(CommandError::SessionNotFound(id))?;
    Ok(session.resize(TerminalSize { rows, cols })?)
}

/// Stops the session's script (if still running) and forgets the session.
#[tauri::command]
pub async fn close_terminal(id: SessionId, state: State<'_, AppState>) -> Result<(), CommandError> {
    // Dropping the session kills its child; see `Session::drop`.
    lock(&state.sessions).remove(&id);
    Ok(())
}
