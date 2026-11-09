//! Shell tabs: the user's interactive shell in a folder of an open project, optionally with
//! a script's command line already typed at the prompt. Mirrors `src/lib/ipc/terminal.ts`.

use runbook_core::PackageManager;
use runbook_process::{SessionId, SpawnOptions, TerminalSize, command_line, user_shell};
use serde::Serialize;
use tauri::State;
use tauri::ipc::{Channel, InvokeResponseBody};

use crate::error::CommandError;
use crate::state::AppState;
use crate::terminal::{ExitDto, script_environment, start_session};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellDto {
    id: SessionId,
    /// Text for the frontend to type at the first prompt, without a newline.
    prefill: Option<String>,
}

/// Opens the user's shell in the package at `package_dir` (relative to `project_root`; empty
/// for the project root). With `script`, which must exist in that package, the reply also
/// carries `<package manager> run <script> ` for the frontend to type at the prompt; the
/// user adds arguments and runs it. Output and exit work as in `run_script`.
#[allow(clippy::too_many_arguments)] // Tauri commands take their IPC arguments one by one.
#[tauri::command]
pub async fn open_shell(
    project_root: String,
    package_dir: String,
    script: Option<String>,
    cols: u16,
    rows: u16,
    on_output: Channel<InvokeResponseBody>,
    on_exit: Channel<ExitDto>,
    state: State<'_, AppState>,
) -> Result<ShellDto, CommandError> {
    let (target, prefill) = match &script {
        Some(script) => {
            let target = state
                .projects
                .resolve(&project_root, &package_dir, script)?;
            let prefill = prefill_line(target.package_manager, script);
            (target, Some(prefill))
        }
        None => (
            state
                .projects
                .resolve_package(&project_root, &package_dir)?,
            None,
        ),
    };

    let env = script_environment(&state);
    let shell = user_shell(&env);
    let options = SpawnOptions {
        program: shell.program,
        args: shell.args,
        cwd: target.cwd,
        env,
        size: TerminalSize { rows, cols },
    };
    let id = start_session(&state, project_root, options, on_output, on_exit).await?;
    Ok(ShellDto { id, prefill })
}

/// The line typed at the prompt, ending in a space so the user can type arguments right
/// away. npm reads flags after the script name as its own options, so its line already
/// ends in `--`; pnpm passes them through to the script.
fn prefill_line(package_manager: PackageManager, script: &str) -> String {
    let command = package_manager.command();
    let line = match package_manager {
        PackageManager::Pnpm => command_line(&[command, "run", script]),
        PackageManager::Npm => command_line(&[command, "run", script, "--"]),
    };
    format!("{line} ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pnpm_line_passes_arguments_straight_to_the_script() {
        assert_eq!(
            prefill_line(PackageManager::Pnpm, "db:migrate"),
            "pnpm run db:migrate "
        );
    }

    #[test]
    fn npm_line_ends_with_the_argument_separator() {
        assert_eq!(
            prefill_line(PackageManager::Npm, "db:migrate"),
            "npm run db:migrate -- "
        );
    }

    #[test]
    fn script_names_are_quoted_for_the_shell() {
        assert_eq!(prefill_line(PackageManager::Pnpm, "a b"), "pnpm run 'a b' ");
    }
}
