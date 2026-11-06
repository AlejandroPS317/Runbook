//! Discovery over the projects in `fixtures/` and over generated temporary projects.

use std::fs;
use std::path::{Path, PathBuf};

use runbook_core::{Discovery, DiscoveryError, ManifestIssue, ProblemKind, discover};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn dirs(discovery: &Discovery) -> Vec<&str> {
    discovery
        .packages
        .iter()
        .map(|p| p.relative_dir.as_str())
        .collect()
}

fn scripts_of<'a>(discovery: &'a Discovery, dir: &str) -> Vec<(&'a str, &'a str)> {
    let package = discovery
        .packages
        .iter()
        .find(|p| p.relative_dir == dir)
        .unwrap_or_else(|| panic!("package {dir:?} not discovered"));
    package
        .scripts
        .iter()
        .map(|s| (s.name.as_str(), s.command.as_str()))
        .collect()
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn monorepo_lists_every_package_root_first() {
    let discovery = discover(&fixture("pnpm-turbo-monorepo")).unwrap();
    assert_eq!(
        dirs(&discovery),
        ["", "apps/api", "apps/web", "packages/config", "packages/ui"]
    );
    assert!(discovery.problems.is_empty(), "{:?}", discovery.problems);
}

#[test]
fn monorepo_skips_dependencies_build_output_and_caches() {
    let discovery = discover(&fixture("pnpm-turbo-monorepo")).unwrap();
    let decoys: Vec<_> = discovery
        .packages
        .iter()
        .filter(|p| p.scripts.iter().any(|s| s.name == "must-not-appear"))
        .map(|p| p.relative_dir.clone())
        .collect();
    assert!(decoys.is_empty(), "decoys discovered: {decoys:?}");
}

#[test]
fn monorepo_root_scripts_keep_file_order_and_commands() {
    let discovery = discover(&fixture("pnpm-turbo-monorepo")).unwrap();
    assert_eq!(
        scripts_of(&discovery, ""),
        [
            ("dev", "turbo run dev"),
            ("build", "turbo run build"),
            ("test", "turbo run test"),
            ("lint", "turbo run lint"),
            ("dev:web", "pnpm --filter @acme/web dev"),
            ("dev:api", "pnpm --filter @acme/api dev"),
        ]
    );
}

#[test]
fn monorepo_package_details() {
    let discovery = discover(&fixture("pnpm-turbo-monorepo")).unwrap();
    let web = discovery
        .packages
        .iter()
        .find(|p| p.relative_dir == "apps/web")
        .unwrap();
    assert_eq!(web.name.as_deref(), Some("@acme/web"));
    assert_eq!(web.dir, discovery.root.join("apps").join("web"));
    assert!(web.dir.is_absolute());
    assert_eq!(
        scripts_of(&discovery, "apps/web")
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>(),
        ["dev", "build", "test", "preview"]
    );
    assert!(scripts_of(&discovery, "packages/config").is_empty());
}

#[test]
fn edge_cases_report_problems_and_keep_usable_packages() {
    let discovery = discover(&fixture("edge-cases")).unwrap();
    assert_eq!(
        dirs(&discovery),
        ["", "no-name", "non-string-script", "scripts-not-object"]
    );
    assert_eq!(
        scripts_of(&discovery, "non-string-script"),
        [("ok", "echo ok")]
    );
    assert_eq!(
        scripts_of(&discovery, "no-name"),
        [("start", "node index.js")]
    );

    let problems: Vec<_> = discovery
        .problems
        .iter()
        .map(|p| (p.relative_path.as_str(), &p.kind))
        .collect();
    assert_eq!(problems.len(), 5, "{problems:?}");
    let kind_of = |path: &str| {
        problems
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, k)| (*k).clone())
            .unwrap_or_else(|| panic!("no problem for {path}"))
    };
    assert!(matches!(
        kind_of("empty-file/package.json"),
        ProblemKind::Manifest(ManifestIssue::InvalidJson(_))
    ));
    assert!(matches!(
        kind_of("invalid-json/package.json"),
        ProblemKind::Manifest(ManifestIssue::InvalidJson(_))
    ));
    assert_eq!(
        kind_of("not-an-object/package.json"),
        ProblemKind::Manifest(ManifestIssue::NotAnObject)
    );
    assert_eq!(
        kind_of("scripts-not-object/package.json"),
        ProblemKind::Manifest(ManifestIssue::ScriptsNotAnObject)
    );
    assert_eq!(
        kind_of("non-string-script/package.json"),
        ProblemKind::Manifest(ManifestIssue::NonStringScript("bad".into()))
    );
}

#[test]
fn honors_gitignore_inside_a_git_repository() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir(root.join(".git")).unwrap();
    write(root, ".gitignore", "generated/\n");
    write(root, "package.json", r#"{"scripts": {"dev": "vite"}}"#);
    write(root, "generated/package.json", r#"{"scripts": {"x": "x"}}"#);
    write(
        root,
        "apps/site/package.json",
        r#"{"scripts": {"dev": "vite"}}"#,
    );

    let discovery = discover(root).unwrap();
    assert_eq!(dirs(&discovery), ["", "apps/site"]);
}

#[test]
fn searches_hidden_folders_that_are_not_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        ".github/actions/release/package.json",
        r#"{"scripts": {"build": "ncc build"}}"#,
    );
    let discovery = discover(tmp.path()).unwrap();
    assert_eq!(dirs(&discovery), [".github/actions/release"]);
}

#[test]
fn opened_folder_is_searched_even_if_its_name_is_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("build");
    write(&root, "package.json", r#"{"scripts": {"dev": "vite"}}"#);
    let discovery = discover(&root).unwrap();
    assert_eq!(dirs(&discovery), [""]);
}

#[test]
fn folder_without_packages_is_empty_not_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let discovery = discover(tmp.path()).unwrap();
    assert!(discovery.packages.is_empty());
    assert!(discovery.problems.is_empty());
}

#[test]
fn missing_root_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let err = discover(&tmp.path().join("nope")).unwrap_err();
    assert!(matches!(err, DiscoveryError::NotFound(_)), "{err}");
}

#[test]
fn file_as_root_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "package.json", "{}");
    let err = discover(&tmp.path().join("package.json")).unwrap_err();
    assert!(matches!(err, DiscoveryError::NotADirectory(_)), "{err}");
}

#[test]
fn many_packages_are_all_found_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    for i in 0..500 {
        write(
            tmp.path(),
            &format!("packages/pkg-{i:03}/package.json"),
            r#"{"scripts": {"build": "tsc", "test": "vitest run"}}"#,
        );
        write(
            tmp.path(),
            &format!("packages/pkg-{i:03}/node_modules/dep/package.json"),
            r#"{"scripts": {"must-not-appear": "x"}}"#,
        );
    }
    let discovery = discover(tmp.path()).unwrap();
    assert_eq!(discovery.packages.len(), 500);
    assert_eq!(discovery.packages[0].relative_dir, "packages/pkg-000");
    assert_eq!(discovery.packages[499].relative_dir, "packages/pkg-499");
}
