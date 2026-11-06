//! Tauri shell for Runbook. Keeps only wiring (commands, state, events); logic lives in
//! the crates under `crates/`.

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        // Without an event loop the app cannot show anything, so failing here is fatal.
        .expect("failed to start the Runbook event loop");
}
