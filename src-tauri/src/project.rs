//! Opening a project folder. Mirrors `src/lib/ipc/types.ts`.

use std::path::PathBuf;

use runbook_core::{Discovery, Package, PackageRole, discover};
use runbook_process::DEFAULT_GRACE;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::error::CommandError;
use crate::projects::root_key;
use crate::recent::RecentProjects;
use crate::state::{AppState, lock};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    root: String,
    /// Folder name, used as the project title.
    name: String,
    package_manager: &'static str,
    packages: Vec<PackageDto>,
    problems: Vec<ProblemDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PackageDto {
    relative_dir: String,
    name: Option<String>,
    role: RoleDto,
    scripts: Vec<ScriptDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum RoleDto {
    Root,
    Member,
    Unlisted,
}

#[derive(Serialize)]
struct ScriptDto {
    name: String,
    command: String,
}

#[derive(Serialize)]
struct ProblemDto {
    path: String,
    message: String,
}

/// The folder passed on the command line, if any. Returns it only once.
#[tauri::command]
pub async fn take_initial_path(state: State<'_, AppState>) -> Result<Option<String>, CommandError> {
    Ok(lock(&state.initial_path)
        .take()
        .map(|path| path.display().to_string()))
}

/// Discovers the project at `path` and adds it to the open projects. Opening a project that
/// is already open refreshes its scripts and keeps its running sessions.
#[tauri::command]
pub async fn open_project(
    path: PathBuf,
    state: State<'_, AppState>,
    recent: State<'_, RecentProjects>,
) -> Result<ProjectDto, CommandError> {
    let discovery = discover(&path)?;
    let dto = to_dto(&discovery);
    // A list that cannot be saved (read-only data folder) must not block opening.
    let _ = recent.record(&discovery.root, &dto.name);
    state.projects.open(discovery);
    Ok(dto)
}

/// Closes the project at `root` and stops every script it started, with its process tree.
/// Resolves once they have all exited (at most the grace period plus a short kill wait).
#[tauri::command]
pub async fn close_project(root: String, app: AppHandle) -> Result<(), CommandError> {
    let ids = app.state::<AppState>().projects.close(&root);
    // Stopping blocks for up to the grace period, so it runs on the blocking pool.
    let _ = tauri::async_runtime::spawn_blocking(move || {
        app.state::<AppState>()
            .sessions
            .stop_many(&ids, DEFAULT_GRACE)
    })
    .await;
    Ok(())
}

fn to_dto(discovery: &Discovery) -> ProjectDto {
    ProjectDto {
        root: root_key(discovery),
        name: discovery
            .root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| discovery.root.display().to_string()),
        package_manager: discovery.package_manager.command(),
        packages: discovery.packages.iter().map(package_dto).collect(),
        problems: discovery
            .problems
            .iter()
            .map(|p| ProblemDto {
                path: p.relative_path.clone(),
                message: p.kind.to_string(),
            })
            .collect(),
    }
}

fn package_dto(package: &Package) -> PackageDto {
    PackageDto {
        relative_dir: package.relative_dir.clone(),
        name: package.name.clone(),
        role: match package.role {
            PackageRole::Root => RoleDto::Root,
            PackageRole::Member => RoleDto::Member,
            PackageRole::Unlisted => RoleDto::Unlisted,
        },
        scripts: package
            .scripts
            .iter()
            .map(|s| ScriptDto {
                name: s.name.clone(),
                command: s.command.clone(),
            })
            .collect(),
    }
}
