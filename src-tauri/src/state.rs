//! State shared by all commands.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use runbook_process::{Environment, SessionManager, login_environment};

use crate::git::GitBusy;
use crate::projects::OpenProjects;

#[derive(Default)]
pub struct AppState {
    /// Folder given on the command line (`runbook <folder>`), opened once at startup.
    pub initial_path: Mutex<Option<PathBuf>>,
    pub projects: OpenProjects,
    /// Every terminal session of every project, one per tab.
    pub sessions: SessionManager,
    pub git_busy: GitBusy,
    environment: OnceLock<Environment>,
}

impl AppState {
    /// The user's login-shell environment. The first call reads it (up to a few hundred
    /// milliseconds with nvm); `lib.rs` warms it at startup so scripts start immediately.
    pub fn environment(&self) -> &Environment {
        self.environment.get_or_init(login_environment)
    }
}

/// Locks `mutex`, recovering the data if a previous holder panicked: the project
/// snapshot and paths stay valid even if one command failed half-way.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
