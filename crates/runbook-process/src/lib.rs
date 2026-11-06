//! Process control for Runbook: running scripts in pseudo-terminals with the user's shell
//! environment. No Tauri dependency; tested with `cargo test`.

mod env;
mod session;

pub use env::{Environment, login_environment};
pub use session::{ExitInfo, Session, SessionError, SessionEvent, SpawnOptions, TerminalSize};
