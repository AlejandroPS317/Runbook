//! The projects open in the window and the terminal sessions each one started. Closing a
//! project must stop exactly its own sessions, so every session is attached to the project
//! it was started from. No Tauri types here, so it is unit-tested without a window.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;

use runbook_core::{Discovery, PackageManager};
use runbook_process::SessionId;

use crate::error::CommandError;
use crate::state::lock;

/// Identifies an open project; it is also the `root` the frontend receives.
pub fn root_key(discovery: &Discovery) -> String {
    discovery.root.display().to_string()
}

/// Where and how to run commands for one package.
#[derive(Debug, PartialEq, Eq)]
pub struct PackageTarget {
    pub cwd: PathBuf,
    pub package_manager: PackageManager,
}

#[derive(Default)]
pub struct OpenProjects {
    projects: Mutex<HashMap<String, OpenProject>>,
}

struct OpenProject {
    discovery: Discovery,
    sessions: HashSet<SessionId>,
}

impl OpenProjects {
    /// Adds the project, or refreshes its snapshot when it is already open (its running
    /// sessions stay attached). Returns its root key.
    pub fn open(&self, discovery: Discovery) -> String {
        let root = root_key(&discovery);
        let mut projects = lock(&self.projects);
        match projects.get_mut(&root) {
            Some(open) => open.discovery = discovery,
            None => {
                projects.insert(
                    root.clone(),
                    OpenProject {
                        discovery,
                        sessions: HashSet::new(),
                    },
                );
            }
        }
        root
    }

    /// Replaces the snapshot of a project that is still open, keeping its sessions. Returns
    /// `false`, and changes nothing, when it was closed in the meantime.
    pub fn refresh(&self, discovery: Discovery) -> bool {
        match lock(&self.projects).get_mut(&root_key(&discovery)) {
            Some(open) => {
                open.discovery = discovery;
                true
            }
            None => false,
        }
    }

    /// Forgets the project and returns the sessions it started, for the caller to stop.
    pub fn close(&self, root: &str) -> Vec<SessionId> {
        lock(&self.projects)
            .remove(root)
            .map(|open| open.sessions.into_iter().collect())
            .unwrap_or_default()
    }

    /// Checks that `script` exists in the package at `package_dir` of the open project.
    pub fn resolve(
        &self,
        root: &str,
        package_dir: &str,
        script: &str,
    ) -> Result<PackageTarget, CommandError> {
        self.lookup(root, package_dir, Some(script))
    }

    /// The folder of the package at `package_dir` of the open project. An empty
    /// `package_dir` is the project root, even when the root has no `package.json`.
    pub fn resolve_package(
        &self,
        root: &str,
        package_dir: &str,
    ) -> Result<PackageTarget, CommandError> {
        self.lookup(root, package_dir, None)
    }

    fn lookup(
        &self,
        root: &str,
        package_dir: &str,
        script: Option<&str>,
    ) -> Result<PackageTarget, CommandError> {
        let projects = lock(&self.projects);
        let discovery = &projects
            .get(root)
            .ok_or_else(|| CommandError::ProjectNotOpen(root.to_owned()))?
            .discovery;
        let package = discovery
            .packages
            .iter()
            .find(|p| p.relative_dir == package_dir);
        let cwd = match (package, script) {
            (Some(package), Some(script)) if !package.scripts.iter().any(|s| s.name == script) => {
                return Err(CommandError::ScriptNotFound {
                    package: package_dir.to_owned(),
                    script: script.to_owned(),
                });
            }
            (Some(package), _) => package.dir.clone(),
            (None, None) if package_dir.is_empty() => discovery.root.clone(),
            (None, _) => return Err(CommandError::PackageNotFound(package_dir.to_owned())),
        };
        Ok(PackageTarget {
            cwd,
            package_manager: discovery.package_manager,
        })
    }

    /// Records that the project at `root` started session `id`. Returns `false` when the
    /// project was closed in the meantime; the caller must then stop the session itself.
    pub fn attach(&self, root: &str, id: SessionId) -> bool {
        match lock(&self.projects).get_mut(root) {
            Some(open) => {
                open.sessions.insert(id);
                true
            }
            None => false,
        }
    }

    /// Forgets session `id`, whichever project started it (its tab was closed).
    pub fn detach(&self, id: SessionId) {
        for open in lock(&self.projects).values_mut() {
            if open.sessions.remove(&id) {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use runbook_core::discover;

    use super::*;

    fn fixture(name: &str) -> Discovery {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures")
            .join(name);
        discover(&path).expect("fixture discovers")
    }

    fn two_projects() -> (OpenProjects, String, String) {
        let projects = OpenProjects::default();
        let pnpm = projects.open(fixture("pnpm-turbo-monorepo"));
        let npm = projects.open(fixture("npm-workspaces"));
        (projects, pnpm, npm)
    }

    #[test]
    fn opening_a_second_project_keeps_the_first() {
        let (projects, pnpm, npm) = two_projects();
        assert_ne!(pnpm, npm);
        assert!(projects.resolve(&pnpm, "apps/web", "dev").is_ok());
        assert!(projects.resolve(&npm, "", "build").is_ok());
    }

    #[test]
    fn scripts_resolve_within_their_own_project() {
        let (projects, pnpm, npm) = two_projects();
        let target = projects.resolve(&pnpm, "apps/web", "dev").unwrap();
        assert_eq!(target.package_manager, PackageManager::Pnpm);
        assert!(target.cwd.ends_with("apps/web"));
        assert!(matches!(
            projects.resolve(&npm, "apps/web", "dev"),
            Err(CommandError::PackageNotFound(_))
        ));
        assert!(matches!(
            projects.resolve(&pnpm, "apps/web", "no-such-script"),
            Err(CommandError::ScriptNotFound { .. })
        ));
    }

    #[test]
    fn packages_resolve_to_their_folder_without_a_script() {
        let (projects, pnpm, npm) = two_projects();
        let target = projects.resolve_package(&pnpm, "apps/web").unwrap();
        assert!(target.cwd.ends_with("apps/web"));
        assert_eq!(target.package_manager, PackageManager::Pnpm);
        assert!(matches!(
            projects.resolve_package(&npm, "apps/web"),
            Err(CommandError::PackageNotFound(_))
        ));
    }

    #[test]
    fn the_root_resolves_even_without_a_root_package() {
        let projects = OpenProjects::default();
        let mut discovery = fixture("pnpm-turbo-monorepo");
        discovery.packages.retain(|p| !p.relative_dir.is_empty());
        let root = projects.open(discovery);
        let target = projects.resolve_package(&root, "").unwrap();
        assert_eq!(target.cwd.display().to_string(), root);
        assert!(matches!(
            projects.resolve(&root, "", "dev"),
            Err(CommandError::PackageNotFound(_))
        ));
    }

    #[test]
    fn a_root_that_is_not_open_is_rejected() {
        let (projects, pnpm, _) = two_projects();
        projects.close(&pnpm);
        assert!(matches!(
            projects.resolve(&pnpm, "apps/web", "dev"),
            Err(CommandError::ProjectNotOpen(_))
        ));
    }

    #[test]
    fn closing_returns_only_that_projects_sessions() {
        let (projects, pnpm, npm) = two_projects();
        assert!(projects.attach(&pnpm, 1));
        assert!(projects.attach(&npm, 2));
        assert!(projects.attach(&pnpm, 3));
        projects.detach(3);

        assert_eq!(projects.close(&pnpm), vec![1]);
        assert!(projects.resolve(&npm, "", "build").is_ok());
        assert_eq!(projects.close(&npm), vec![2]);
        assert!(projects.close(&npm).is_empty());
    }

    #[test]
    fn attaching_to_a_closed_project_fails() {
        let (projects, pnpm, _) = two_projects();
        projects.close(&pnpm);
        assert!(!projects.attach(&pnpm, 1));
    }

    #[test]
    fn refreshing_never_reopens_a_closed_project() {
        let (projects, pnpm, _) = two_projects();
        projects.attach(&pnpm, 4);
        assert!(projects.refresh(fixture("pnpm-turbo-monorepo")));
        assert_eq!(projects.close(&pnpm), vec![4]);
        assert!(!projects.refresh(fixture("pnpm-turbo-monorepo")));
        assert!(matches!(
            projects.resolve(&pnpm, "apps/web", "dev"),
            Err(CommandError::ProjectNotOpen(_))
        ));
    }

    #[test]
    fn reopening_keeps_attached_sessions() {
        let (projects, pnpm, _) = two_projects();
        projects.attach(&pnpm, 7);
        assert_eq!(projects.open(fixture("pnpm-turbo-monorepo")), pnpm);
        assert_eq!(projects.close(&pnpm), vec![7]);
    }
}
