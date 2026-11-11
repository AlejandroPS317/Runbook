use std::io;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("git is not installed or not on PATH")]
    NotInstalled,
    #[error("cannot run git: {0}")]
    Spawn(#[source] io::Error),
    /// Git ran and failed; the message is Git's own, without its `hint:` lines.
    #[error("{0}")]
    Failed(String),
    #[error("git did not finish within {} seconds", .0.as_secs())]
    TimedOut(Duration),
    #[error("unexpected git output: {0}")]
    Parse(String),
    #[error("invalid branch name \"{0}\"")]
    InvalidBranch(String),
}
