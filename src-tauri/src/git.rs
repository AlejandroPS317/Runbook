//! Git commands behind the status bar's branch menu. Mirrors `src/lib/ipc/git.ts`.
//!
//! Git runs on the blocking pool: a fetch or pull may take as long as the network does.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use runbook_git::{Branch, Repository, Status};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::error::CommandError;
use crate::project::{ProjectDto, rediscover};
use crate::state::{AppState, lock};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusDto {
    branch: Option<String>,
    commit: Option<String>,
    upstream: Option<String>,
    ahead: u32,
    behind: u32,
    changes: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchDto {
    name: String,
    remote: Option<String>,
    current: bool,
}

/// Result of a switch or pull: the branch moved, so the scripts were read again.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitChangeDto {
    status: GitStatusDto,
    /// `None` when the project could not be read again; the Git operation still happened.
    project: Option<ProjectDto>,
    discovery_error: Option<String>,
}

/// Projects with a switch, fetch or pull in progress: one at a time per project.
#[derive(Default)]
pub struct GitBusy(Mutex<HashSet<String>>);

/// Releases the project's slot when the operation ends, however it ends.
struct BusyGuard<'a> {
    busy: &'a GitBusy,
    root: String,
}

impl GitBusy {
    fn claim(&self, root: &str) -> Result<BusyGuard<'_>, CommandError> {
        if !lock(&self.0).insert(root.to_owned()) {
            return Err(CommandError::GitBusy);
        }
        Ok(BusyGuard {
            busy: self,
            root: root.to_owned(),
        })
    }
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        lock(&self.busy.0).remove(&self.root);
    }
}

/// The project's Git status, or `None` when its root is not the top level of a repository
/// or `git` cannot run; the branch item is then hidden.
#[tauri::command]
pub async fn git_status(
    root: String,
    app: AppHandle,
) -> Result<Option<GitStatusDto>, CommandError> {
    blocking(app, move |state| {
        let dir = state.projects.resolve_package(&root, "")?.cwd;
        let repo = Repository::new(&dir, state.environment());
        if !repo.is_top_level().unwrap_or(false) {
            return Ok(None);
        }
        Ok(Some(status_dto(repo.status()?)))
    })
    .await
}

#[tauri::command]
pub async fn git_branches(root: String, app: AppHandle) -> Result<Vec<BranchDto>, CommandError> {
    blocking(app, move |state| {
        let dir = repository_dir(state, &root)?;
        let branches = Repository::new(&dir, state.environment()).branches()?;
        Ok(branches.into_iter().map(branch_dto).collect())
    })
    .await
}

/// Switches branch (see `Repository::switch`) and reads the project's scripts again.
#[tauri::command]
pub async fn git_switch(
    root: String,
    name: String,
    remote: Option<String>,
    app: AppHandle,
) -> Result<GitChangeDto, CommandError> {
    blocking(app, move |state| {
        let _busy = state.git_busy.claim(&root)?;
        let dir = repository_dir(state, &root)?;
        Repository::new(&dir, state.environment()).switch(&name, remote.as_deref())?;
        changed(state, &dir)
    })
    .await
}

/// Fetches every remote; returns the status with the new ahead and behind counts.
#[tauri::command]
pub async fn git_fetch(root: String, app: AppHandle) -> Result<GitStatusDto, CommandError> {
    blocking(app, move |state| {
        let _busy = state.git_busy.claim(&root)?;
        let dir = repository_dir(state, &root)?;
        let repo = Repository::new(&dir, state.environment());
        repo.fetch()?;
        Ok(status_dto(repo.status()?))
    })
    .await
}

/// Fast-forwards the current branch and reads the project's scripts again.
#[tauri::command]
pub async fn git_pull(root: String, app: AppHandle) -> Result<GitChangeDto, CommandError> {
    blocking(app, move |state| {
        let _busy = state.git_busy.claim(&root)?;
        let dir = repository_dir(state, &root)?;
        Repository::new(&dir, state.environment()).pull()?;
        changed(state, &dir)
    })
    .await
}

async fn blocking<T: Send + 'static>(
    app: AppHandle,
    task: impl FnOnce(&AppState) -> Result<T, CommandError> + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(move || task(&app.state::<AppState>()))
        .await
        .map_err(|_| CommandError::GitInterrupted)?
}

/// The open project's folder, which must be the top level of a repository.
fn repository_dir(state: &AppState, root: &str) -> Result<PathBuf, CommandError> {
    let dir = state.projects.resolve_package(root, "")?.cwd;
    if !Repository::new(&dir, state.environment()).is_top_level()? {
        return Err(CommandError::NotARepository(root.to_owned()));
    }
    Ok(dir)
}

/// After the branch moved: the new status, and the project read again. A project that no
/// longer reads (a broken `package.json` on the new branch) is reported, not an error.
fn changed(state: &AppState, dir: &Path) -> Result<GitChangeDto, CommandError> {
    let status = status_dto(Repository::new(dir, state.environment()).status()?);
    let (project, discovery_error) = match rediscover(&state.projects, dir) {
        Ok(project) => (Some(project), None),
        Err(err) => (None, Some(err.to_string())),
    };
    Ok(GitChangeDto {
        status,
        project,
        discovery_error,
    })
}

fn status_dto(status: Status) -> GitStatusDto {
    GitStatusDto {
        branch: status.branch,
        commit: status.commit,
        upstream: status.upstream,
        ahead: status.ahead,
        behind: status.behind,
        changes: status.changes,
    }
}

fn branch_dto(branch: Branch) -> BranchDto {
    BranchDto {
        name: branch.name,
        remote: branch.remote,
        current: branch.current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_git_operation_per_project_at_a_time() {
        let busy = GitBusy::default();
        let first = busy.claim("/a").unwrap();
        assert!(matches!(busy.claim("/a"), Err(CommandError::GitBusy)));
        assert!(busy.claim("/b").is_ok());
        drop(first);
        assert!(busy.claim("/a").is_ok());
    }
}
