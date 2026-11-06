//! State shared by all commands.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use runbook_core::Discovery;
use runbook_process::{Environment, Session, login_environment};

pub type SessionId = u32;

#[derive(Default)]
pub struct AppState {
    /// Folder given on the command line (`runbook <folder>`), opened once at startup.
    pub initial_path: Mutex<Option<PathBuf>>,
    pub project: Mutex<Option<Discovery>>,
    pub sessions: Mutex<HashMap<SessionId, Session>>,
    next_session: AtomicU32,
    environment: OnceLock<Environment>,
}

impl AppState {
    pub fn next_session_id(&self) -> SessionId {
        self.next_session.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// The user's login-shell environment. The first call reads it (up to a few hundred
    /// milliseconds with nvm); `lib.rs` warms it at startup so scripts start immediately.
    pub fn environment(&self) -> &Environment {
        self.environment.get_or_init(login_environment)
    }
}

/// Locks `mutex`, recovering the data if a previous holder panicked: the maps and the
/// project snapshot stay valid even if one command failed half-way.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
