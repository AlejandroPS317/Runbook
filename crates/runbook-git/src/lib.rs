//! Basic Git branch control for Runbook: read the current branch, list branches, switch,
//! fetch and pull, by running the user's own `git` (ADR 0008). No Tauri dependency; tested
//! with `cargo test` against temporary repositories.

mod branches;
mod command;
mod error;
mod repository;
mod status;

pub use branches::Branch;
pub use error::GitError;
pub use repository::{Environment, NETWORK_TIMEOUT, Repository};
pub use status::Status;
