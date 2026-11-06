//! Workspace resolution (pnpm and npm), package roles and package-manager detection.

use std::fs;
use std::path::{Path, PathBuf};

use runbook_core::{
    Discovery, PackageManager, PackageRole, ProblemKind, WorkspaceIssue, WorkspaceSource, discover,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn roles(discovery: &Discovery) -> Vec<(&str, PackageRole)> {
    discovery
        .packages
        .iter()
        .map(|p| (p.relative_dir.as_str(), p.role))
        .collect()
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn pnpm_monorepo_classifies_members_and_unlisted_packages() {
    let discovery = discover(&fixture("pnpm-turbo-monorepo")).unwrap();
    assert_eq!(discovery.package_manager, PackageManager::Pnpm);
    let workspaces = discovery.workspaces.as_ref().unwrap();
    assert_eq!(workspaces.source, WorkspaceSource::PnpmWorkspaceYaml);
    assert_eq!(
        workspaces.patterns,
        ["apps/*", "packages/*", "!packages/legacy"]
    );
    assert_eq!(
        roles(&discovery),
        [
            ("", PackageRole::Root),
            ("apps/api", PackageRole::Member),
            ("apps/web", PackageRole::Member),
            ("examples/demo", PackageRole::Unlisted),
            ("packages/config", PackageRole::Member),
            ("packages/legacy", PackageRole::Unlisted),
            ("packages/ui", PackageRole::Member),
        ]
    );
    assert!(discovery.problems.is_empty(), "{:?}", discovery.problems);
}

#[test]
fn npm_workspaces_include_members_inside_normally_skipped_folders() {
    let discovery = discover(&fixture("npm-workspaces")).unwrap();
    assert_eq!(discovery.package_manager, PackageManager::Npm);
    let workspaces = discovery.workspaces.as_ref().unwrap();
    assert_eq!(workspaces.source, WorkspaceSource::PackageJson);
    assert_eq!(
        roles(&discovery),
        [
            ("", PackageRole::Root),
            ("docs/site", PackageRole::Unlisted),
            ("packages/cli", PackageRole::Member),
            ("packages/core", PackageRole::Member),
            ("tools/build", PackageRole::Member),
        ]
    );
    assert!(discovery.problems.is_empty(), "{:?}", discovery.problems);
}

#[test]
fn without_workspaces_every_other_package_is_unlisted() {
    let discovery = discover(&fixture("edge-cases")).unwrap();
    assert_eq!(discovery.workspaces, None);
    assert_eq!(discovery.package_manager, PackageManager::Npm);
    assert!(
        discovery
            .packages
            .iter()
            .all(|p| p.role == PackageRole::Unlisted || p.relative_dir.is_empty())
    );
}

#[test]
fn pnpm_ignores_package_json_workspaces() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    write(
        tmp.path(),
        "package.json",
        r#"{"workspaces": ["packages/*"]}"#,
    );
    write(tmp.path(), "packages/a/package.json", "{}");

    let discovery = discover(tmp.path()).unwrap();
    assert_eq!(discovery.package_manager, PackageManager::Pnpm);
    assert_eq!(discovery.workspaces, None);
    assert_eq!(
        roles(&discovery),
        [
            ("", PackageRole::Root),
            ("packages/a", PackageRole::Unlisted)
        ]
    );
}

#[test]
fn npm_object_form_workspaces() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "package.json",
        r#"{"workspaces": {"packages": ["packages/*"]}}"#,
    );
    write(tmp.path(), "packages/a/package.json", "{}");
    let discovery = discover(tmp.path()).unwrap();
    assert_eq!(
        roles(&discovery),
        [("", PackageRole::Root), ("packages/a", PackageRole::Member)]
    );
}

#[test]
fn invalid_pnpm_workspace_yaml_is_a_problem_not_a_failure() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "pnpm-workspace.yaml", "packages: [apps/*\n");
    write(
        tmp.path(),
        "package.json",
        r#"{"scripts": {"dev": "turbo run dev"}}"#,
    );
    write(tmp.path(), "apps/web/package.json", "{}");

    let discovery = discover(tmp.path()).unwrap();
    assert_eq!(discovery.package_manager, PackageManager::Pnpm);
    assert_eq!(discovery.workspaces, None);
    assert_eq!(discovery.packages.len(), 2);
    assert_eq!(discovery.problems.len(), 1);
    assert_eq!(discovery.problems[0].relative_path, "pnpm-workspace.yaml");
    assert!(matches!(
        discovery.problems[0].kind,
        ProblemKind::Workspace(WorkspaceIssue::InvalidYaml(_))
    ));
}

#[test]
fn invalid_workspace_pattern_is_reported_with_its_file() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "package.json",
        r#"{"workspaces": ["packages/[", "apps/*"]}"#,
    );
    write(tmp.path(), "apps/web/package.json", "{}");

    let discovery = discover(tmp.path()).unwrap();
    assert_eq!(
        roles(&discovery),
        [("", PackageRole::Root), ("apps/web", PackageRole::Member)]
    );
    assert_eq!(discovery.problems.len(), 1);
    assert_eq!(discovery.problems[0].relative_path, "package.json");
    assert!(matches!(
        discovery.problems[0].kind,
        ProblemKind::Workspace(WorkspaceIssue::InvalidPattern { .. })
    ));
}

#[test]
fn unsupported_package_manager_is_reported_and_falls_back() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "package.json",
        r#"{"packageManager": "yarn@4.5.0"}"#,
    );

    let discovery = discover(tmp.path()).unwrap();
    assert_eq!(discovery.package_manager, PackageManager::Npm);
    assert_eq!(
        discovery.problems[0].kind,
        ProblemKind::UnsupportedPackageManager {
            name: "yarn".into(),
            fallback: PackageManager::Npm
        }
    );
}

/// Acceptance test against the user's real pnpm + turbo monorepo.
/// Run with: `RUNBOOK_ACCEPTANCE_REPO=/path/to/repo cargo test -p runbook-core -- --ignored`
#[test]
#[ignore = "needs RUNBOOK_ACCEPTANCE_REPO pointing at a real pnpm + turbo monorepo"]
fn acceptance_real_pnpm_turbo_monorepo() {
    let root = std::env::var_os("RUNBOOK_ACCEPTANCE_REPO")
        .map(PathBuf::from)
        .expect("set RUNBOOK_ACCEPTANCE_REPO");
    let discovery = discover(&root).unwrap();

    assert_eq!(discovery.package_manager, PackageManager::Pnpm);
    let workspaces = discovery.workspaces.as_ref().expect("pnpm workspaces");
    assert_eq!(workspaces.source, WorkspaceSource::PnpmWorkspaceYaml);
    assert!(discovery.problems.is_empty(), "{:?}", discovery.problems);

    let root_package = &discovery.packages[0];
    assert_eq!(root_package.role, PackageRole::Root);
    assert!(
        root_package
            .scripts
            .iter()
            .any(|s| s.command.contains("turbo run")),
        "root scripts should call turbo"
    );
    assert!(
        root_package
            .scripts
            .iter()
            .any(|s| s.command.contains("--filter")),
        "root scripts should use pnpm filters"
    );

    let members: Vec<_> = discovery
        .packages
        .iter()
        .filter(|p| p.role == PackageRole::Member)
        .collect();
    assert!(
        members.iter().any(|p| p.relative_dir.starts_with("apps/")),
        "expected members in apps/"
    );
    assert!(
        members
            .iter()
            .any(|p| p.relative_dir.starts_with("packages/")),
        "expected members in packages/"
    );
    for member in &members {
        assert!(member.name.is_some(), "{} has no name", member.relative_dir);
    }
}
