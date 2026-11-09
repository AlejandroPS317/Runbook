//! Errors returned by commands. The frontend receives them as plain message strings.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error(transparent)]
    Discovery(#[from] runbook_core::DiscoveryError),
    #[error(transparent)]
    Session(#[from] runbook_process::SessionError),
    #[error("no project is open")]
    NoProject,
    #[error("package \"{0}\" is not part of the open project")]
    PackageNotFound(String),
    #[error("script \"{script}\" does not exist in package \"{package}\"")]
    ScriptNotFound { package: String, script: String },
    #[error("terminal session {0} does not exist")]
    SessionNotFound(u32),
    #[error("cannot save the recent projects list: {0}")]
    RecentProjects(#[from] std::io::Error),
    #[error("update failed: {0}")]
    Updater(#[from] tauri_plugin_updater::Error),
    #[error("there is no update to install; check for updates first")]
    NoPendingUpdate,
    #[error("the update install was interrupted")]
    UpdateInterrupted,
}

impl Serialize for CommandError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
