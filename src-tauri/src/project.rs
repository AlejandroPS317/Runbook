//! Opening a project folder. Mirrors `src/lib/ipc/types.ts`.

use std::path::PathBuf;

use runbook_core::{Discovery, Package, PackageRole, discover};
use serde::Serialize;
use tauri::State;

use crate::error::CommandError;
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

/// Discovers the project at `path` and makes it the open project.
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
    *lock(&state.project) = Some(discovery);
    Ok(dto)
}

fn to_dto(discovery: &Discovery) -> ProjectDto {
    ProjectDto {
        root: discovery.root.display().to_string(),
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
