//! Tauri shell for Runbook. Keeps only wiring (commands, state, events); logic lives in
//! the crates under `crates/`.

mod error;
mod project;
mod recent;
mod state;
mod terminal;

use std::path::PathBuf;
use std::thread;

use tauri::{Manager, RunEvent};

use crate::state::{AppState, lock};

pub fn run() {
    let state = AppState::default();
    *lock(&state.initial_path) = std::env::args_os().nth(1).map(PathBuf::from);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
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
            recent::recent_projects,
            recent::forget_recent_project,
            terminal::run_script,
            terminal::write_terminal,
            terminal::resize_terminal,
            terminal::stop_terminal,
            terminal::close_terminal,
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
