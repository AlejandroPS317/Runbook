// Release builds on Windows run without an attached console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    runbook_app::run();
}
