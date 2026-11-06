//! Project discovery: every `package.json` under a folder, with its scripts, grouped by
//! package and classified against the root's workspace definition.
//!
//! Discovery never stops at a bad file. Unreadable or malformed files become [`Problem`]s
//! next to the packages that were read successfully.

use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::{ManifestIssue, Script, parse_manifest};
use crate::package_manager::PackageManager;
use crate::paths;
use crate::root_config;
use crate::walk::find_manifests;
use crate::workspace::{WorkspaceIssue, WorkspaceMatcher, Workspaces};

/// How a package relates to the project root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageRole {
    /// The `package.json` of the opened folder.
    Root,
    /// Declared by the root's workspace patterns.
    Member,
    /// Found on disk but not declared as a workspace (or the root declares none).
    Unlisted,
}

/// A folder with a usable `package.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Absolute path of the folder containing `package.json`; its scripts run here.
    pub dir: PathBuf,
    /// `dir` relative to the project root, `/`-separated on every OS; empty for the root.
    pub relative_dir: String,
    /// The `name` field, when present.
    pub name: Option<String>,
    /// Scripts in file order.
    pub scripts: Vec<Script>,
    pub role: PackageRole,
}

/// Something that went wrong for one path. The rest of the project is still discovered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// Path relative to the project root, `/`-separated; empty for the root itself.
    pub relative_path: String,
    pub kind: ProblemKind,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProblemKind {
    #[error("cannot read file: {0}")]
    Unreadable(String),
    #[error("cannot search folder: {0}")]
    Walk(String),
    #[error(transparent)]
    Manifest(#[from] ManifestIssue),
    #[error(transparent)]
    Workspace(#[from] WorkspaceIssue),
    #[error("package manager \"{name}\" is not supported yet; using {fallback}")]
    UnsupportedPackageManager {
        name: String,
        fallback: PackageManager,
    },
}

/// Everything found under a project folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    /// Absolute path of the folder that was opened.
    pub root: PathBuf,
    /// The manager that runs this project's scripts.
    pub package_manager: PackageManager,
    /// The root's workspace patterns, when it declares any.
    pub workspaces: Option<Workspaces>,
    /// Packages sorted by `relative_dir`, root first.
    pub packages: Vec<Package>,
    /// Problems sorted by `relative_path`.
    pub problems: Vec<Problem>,
}

/// Failures that prevent discovery from starting at all.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("folder not found: {}", .0.display())]
    NotFound(PathBuf),
    #[error("not a folder: {}", .0.display())]
    NotADirectory(PathBuf),
    #[error("cannot open folder {}: {source}", .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Finds every `package.json` under `root`, reads its scripts and classifies it against the
/// root's workspaces.
pub fn discover(root: &Path) -> Result<Discovery, DiscoveryError> {
    let root = open_root(root)?;
    let config = root_config::read(&root);
    let walked = find_manifests(&root, config.matcher.explicit_dirs().clone());

    let mut problems = config.problems;
    problems.extend(walked.errors.into_iter().map(|(path, message)| Problem {
        relative_path: path.map(|p| paths::relative(&root, &p)).unwrap_or_default(),
        kind: ProblemKind::Walk(message),
    }));

    let mut packages = Vec::new();
    for manifest_path in walked.manifests {
        let relative_path = paths::relative(&root, &manifest_path);
        match read_package(&root, &manifest_path, &config.matcher) {
            Ok((package, issues)) => {
                problems.extend(issues.into_iter().map(|issue| Problem {
                    relative_path: relative_path.clone(),
                    kind: issue.into(),
                }));
                packages.push(package);
            }
            Err(kind) => problems.push(Problem {
                relative_path,
                kind,
            }),
        }
    }

    packages.sort_by(|a, b| paths::compare(&a.relative_dir, &b.relative_dir));
    problems.sort_by(|a, b| paths::compare(&a.relative_path, &b.relative_path));
    Ok(Discovery {
        root,
        package_manager: config.package_manager,
        workspaces: config.workspaces,
        packages,
        problems,
    })
}

fn open_root(root: &Path) -> Result<PathBuf, DiscoveryError> {
    // `absolute` instead of `canonicalize`: keeps the path the user chose (no symlink
    // resolution) and avoids Windows `\\?\` verbatim paths, which some tools reject as cwd.
    let root = std::path::absolute(root).map_err(|source| DiscoveryError::Io {
        path: root.to_path_buf(),
        source,
    })?;
    match std::fs::metadata(&root) {
        Ok(meta) if meta.is_dir() => Ok(root),
        Ok(_) => Err(DiscoveryError::NotADirectory(root)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(DiscoveryError::NotFound(root)),
        Err(source) => Err(DiscoveryError::Io { path: root, source }),
    }
}

fn read_package(
    root: &Path,
    manifest_path: &Path,
    matcher: &WorkspaceMatcher,
) -> Result<(Package, Vec<ManifestIssue>), ProblemKind> {
    let text = std::fs::read_to_string(manifest_path)
        .map_err(|err| ProblemKind::Unreadable(err.to_string()))?;
    let parsed = parse_manifest(&text)?;
    let dir = manifest_path
        .parent()
        .map_or_else(|| root.to_path_buf(), Path::to_path_buf);
    let relative_dir = paths::relative(root, &dir);
    let role = if relative_dir.is_empty() {
        PackageRole::Root
    } else if matcher.is_member(&relative_dir) {
        PackageRole::Member
    } else {
        PackageRole::Unlisted
    };
    let package = Package {
        relative_dir,
        dir,
        name: parsed.manifest.name,
        scripts: parsed.manifest.scripts,
        role,
    };
    Ok((package, parsed.issues))
}
