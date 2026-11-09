//! Process control for Runbook: running scripts in pseudo-terminals with the user's shell
//! environment, and stopping them with their whole process tree. No Tauri dependency;
//! tested with `cargo test`.

mod env;
mod manager;
mod session;
mod stop;
mod tree;

pub use env::{Environment, login_environment};
pub use manager::{SessionId, SessionManager};
pub use session::{ExitInfo, Session, SessionError, SessionEvent, SpawnOptions, TerminalSize};
pub use tree::{DEFAULT_GRACE, KILL_WAIT, StopOutcome};
