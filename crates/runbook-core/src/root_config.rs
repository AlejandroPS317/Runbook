//! Project-level settings read from the root folder before walking: the package manager
//! and the workspace patterns.

use std::fs;
use std::io;
use std::path::Path;

use crate::discovery::{Problem, ProblemKind};
use crate::manifest::parse_manifest;
use crate::package_manager::{self, PackageManager};
use crate::workspace::{
    PNPM_WORKSPACE_FILE, WorkspaceMatcher, WorkspaceSource, Workspaces, parse_pnpm_workspace,
};

const ROOT_MANIFEST: &str = "package.json";

pub struct RootConfig {
    pub package_manager: PackageManager,
    pub workspaces: Option<Workspaces>,
    pub matcher: WorkspaceMatcher,
    pub problems: Vec<Problem>,
}

/// Reads the root's package manager and workspaces.
///
/// pnpm takes workspaces only from `pnpm-workspace.yaml`; npm only from the root
/// `package.json`. Runbook follows the same rule for the detected manager.
pub fn read(root: &Path) -> RootConfig {
    // A missing or broken root package.json means "no packageManager and no workspaces".
    // Its parse problems are reported once, by the walk, like any other manifest.
    let manifest = fs::read_to_string(root.join(ROOT_MANIFEST))
        .ok()
        .and_then(|text| parse_manifest(&text).ok())
        .map(|parsed| parsed.manifest)
        .unwrap_or_default();

    let mut problems = Vec::new();
    let detection = package_manager::detect(root, manifest.package_manager.as_deref());
    if let Some(name) = detection.unsupported {
        problems.push(Problem {
            relative_path: ROOT_MANIFEST.into(),
            kind: ProblemKind::UnsupportedPackageManager {
                name,
                fallback: detection.manager,
            },
        });
    }

    let workspaces = match detection.manager {
        PackageManager::Pnpm => read_pnpm_workspaces(root, &mut problems),
        PackageManager::Npm => manifest.workspaces.map(|patterns| Workspaces {
            source: WorkspaceSource::PackageJson,
            patterns,
        }),
    };

    let patterns = workspaces.as_ref().map_or(&[][..], |w| &w.patterns[..]);
    let (matcher, issues) = WorkspaceMatcher::new(patterns);
    let source_file = match workspaces.as_ref().map(|w| w.source) {
        Some(WorkspaceSource::PnpmWorkspaceYaml) => PNPM_WORKSPACE_FILE,
        _ => ROOT_MANIFEST,
    };
    problems.extend(issues.into_iter().map(|issue| Problem {
        relative_path: source_file.into(),
        kind: issue.into(),
    }));

    RootConfig {
        package_manager: detection.manager,
        workspaces,
        matcher,
        problems,
    }
}

fn read_pnpm_workspaces(root: &Path, problems: &mut Vec<Problem>) -> Option<Workspaces> {
    let text = match fs::read_to_string(root.join(PNPM_WORKSPACE_FILE)) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return None,
        Err(err) => {
            problems.push(Problem {
                relative_path: PNPM_WORKSPACE_FILE.into(),
                kind: ProblemKind::Unreadable(err.to_string()),
            });
            return None;
        }
    };
    match parse_pnpm_workspace(&text) {
        Ok(patterns) => Some(Workspaces {
            source: WorkspaceSource::PnpmWorkspaceYaml,
            patterns,
        }),
        Err(issue) => {
            problems.push(Problem {
                relative_path: PNPM_WORKSPACE_FILE.into(),
                kind: issue.into(),
            });
            None
        }
    }
}
