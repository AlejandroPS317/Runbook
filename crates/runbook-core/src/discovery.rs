//! Project discovery: every `package.json` under a folder, with its scripts.
//!
//! Discovery never stops at a bad file. Unreadable or malformed manifests become
//! [`Problem`]s next to the packages that were read successfully.

use std::cmp::Ordering;
use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::{ManifestIssue, Script, parse_manifest};
use crate::walk::find_manifests;

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
}

/// Everything found under a project folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    /// Absolute path of the folder that was opened.
    pub root: PathBuf,
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

/// Finds every `package.json` under `root` and reads its scripts.
pub fn discover(root: &Path) -> Result<Discovery, DiscoveryError> {
    let root = open_root(root)?;
    let walked = find_manifests(&root);

    let mut packages = Vec::new();
    let mut problems: Vec<Problem> = walked
        .errors
        .into_iter()
        .map(|(path, message)| Problem {
            relative_path: path.map(|p| relative(&root, &p)).unwrap_or_default(),
            kind: ProblemKind::Walk(message),
        })
        .collect();

    for manifest_path in walked.manifests {
        let relative_path = relative(&root, &manifest_path);
        match read_package(&root, &manifest_path) {
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

    packages.sort_by(|a, b| compare_paths(&a.relative_dir, &b.relative_dir));
    problems.sort_by(|a, b| compare_paths(&a.relative_path, &b.relative_path));
    Ok(Discovery {
        root,
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
) -> Result<(Package, Vec<ManifestIssue>), ProblemKind> {
    let text = std::fs::read_to_string(manifest_path)
        .map_err(|err| ProblemKind::Unreadable(err.to_string()))?;
    let parsed = parse_manifest(&text)?;
    let dir = manifest_path
        .parent()
        .map_or_else(|| root.to_path_buf(), Path::to_path_buf);
    let package = Package {
        relative_dir: relative(root, &dir),
        dir,
        name: parsed.manifest.name,
        scripts: parsed.manifest.scripts,
    };
    Ok((package, parsed.issues))
}

/// `path` relative to `root`, joined with `/` so results match across operating systems.
fn relative(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Orders paths component by component, so a folder's children follow the folder itself.
/// The root (`""`) sorts first because its only component is the empty string.
fn compare_paths(a: &str, b: &str) -> Ordering {
    a.split('/').cmp(b.split('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_sorts_before_everything() {
        assert_eq!(compare_paths("", "apps/web"), Ordering::Less);
        assert_eq!(compare_paths("apps/web", ""), Ordering::Greater);
        assert_eq!(compare_paths("", ""), Ordering::Equal);
    }

    #[test]
    fn children_follow_their_parent() {
        let mut dirs = vec!["apps/web-admin", "apps/web/sub", "apps/web", "apps"];
        dirs.sort_by(|a, b| compare_paths(a, b));
        assert_eq!(dirs, ["apps", "apps/web", "apps/web/sub", "apps/web-admin"]);
    }

    #[test]
    fn relative_uses_forward_slashes() {
        let root = Path::new("/project");
        let path = root.join("apps").join("web");
        assert_eq!(relative(root, &path), "apps/web");
        assert_eq!(relative(root, root), "");
    }
}
