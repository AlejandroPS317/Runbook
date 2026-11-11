//! Tauri shell for Runbook. Keeps only wiring (commands, state, events); logic lives in
//! the crates under `crates/`.

mod error;
mod fonts;
mod git;
mod project;
mod projects;
mod recent;
mod shell;
mod state;
mod terminal;
mod updates;

use std::path::PathBuf;
use std::thread;

use tauri::{Manager, RunEvent};

use crate::state::{AppState, lock};

pub fn run() {
    fonts::apply_workaround();
    let state = AppState::default();
    *lock(&state.initial_path) = std::env::args_os().nth(1).map(PathBuf::from);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .manage(updates::PendingUpdate::default())
        .setup(|app| {
            let recent_file = app.path().app_data_dir()?.join("recent-projects.json");
            app.manage(recent::RecentProjects::load(recent_file));
            // Read the login-shell environment in the background so the first script
            // starts without waiting for the user's shell startup files.
            let handle = app.handle().clone();
            thread::spawn(move || {
                handle.state::<AppState>().environment();
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            project::take_initial_path,
            project::open_project,
            project::close_project,
            project::refresh_project,
            git::git_status,
            git::git_branches,
            git::git_switch,
            git::git_fetch,
            git::git_pull,
            recent::recent_projects,
            recent::forget_recent_project,
            terminal::run_script,
            shell::open_shell,
            terminal::write_terminal,
            terminal::resize_terminal,
            terminal::stop_terminal,
            terminal::close_terminal,
            updates::check_for_update,
            updates::running_scripts,
            updates::install_update,
        ])
        .build(tauri::generate_context!())
        // Without an event loop the app cannot show anything, so failing here is fatal.
        .expect("failed to start the Runbook event loop");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            // Stop every script and its process tree before the app process exits.
            handle
                .state::<AppState>()
                .sessions
                .stop_all(runbook_process::DEFAULT_GRACE);
        }
    });
}
