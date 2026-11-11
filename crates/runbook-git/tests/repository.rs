mod common;

use std::fs;

use common::Fixture;
use runbook_git::{Branch, GitError, Repository};

fn names(branches: &[Branch]) -> Vec<(String, Option<String>, bool)> {
    branches
        .iter()
        .map(|b| (b.name.clone(), b.remote.clone(), b.current))
        .collect()
}

#[test]
fn only_the_top_level_of_a_work_tree_counts() {
    let f = Fixture::new();
    let sub = f.work.join("apps");
    fs::create_dir(&sub).unwrap();
    let plain = f.root.join("plain");
    fs::create_dir(&plain).unwrap();

    assert!(Repository::new(&f.work, &f.env).is_top_level().unwrap());
    assert!(!Repository::new(&sub, &f.env).is_top_level().unwrap());
    assert!(!Repository::new(&plain, &f.env).is_top_level().unwrap());
    assert!(
        !Repository::new(&f.root.join("remote.git"), &f.env)
            .is_top_level()
            .unwrap()
    );
}

#[test]
fn status_reports_branch_upstream_and_changes() {
    let f = Fixture::new();
    let repo = Repository::new(&f.work, &f.env);
    let status = repo.status().unwrap();
    assert_eq!(status.branch.as_deref(), Some("main"));
    assert_eq!(status.upstream.as_deref(), Some("origin/main"));
    assert_eq!(status.commit.as_ref().map(String::len), Some(40));
    assert_eq!((status.ahead, status.behind, status.changes), (0, 0, 0));

    fs::write(f.work.join("b.txt"), "edited\n").unwrap();
    fs::write(f.work.join("new.txt"), "new\n").unwrap();
    assert_eq!(repo.status().unwrap().changes, 2);

    f.commit(&f.work, "b.txt", "committed\n");
    let status = repo.status().unwrap();
    assert_eq!((status.ahead, status.changes), (1, 1));
}

#[test]
fn detached_head_has_no_branch() {
    let f = Fixture::new();
    f.git(&f.work, &["switch", "--detach", "HEAD"]);
    let status = Repository::new(&f.work, &f.env).status().unwrap();
    assert_eq!(status.branch, None);
    assert!(status.commit.is_some());
}

#[test]
fn lists_local_branches_then_remote_only_ones() {
    let f = Fixture::new();
    let branches = Repository::new(&f.work, &f.env).branches().unwrap();
    assert_eq!(
        names(&branches),
        vec![
            ("main".into(), None, true),
            ("feature".into(), Some("origin".into()), false),
        ]
    );
}

#[test]
fn switching_to_a_remote_branch_creates_a_tracking_branch() {
    let f = Fixture::new();
    let repo = Repository::new(&f.work, &f.env);
    repo.switch("feature", Some("origin")).unwrap();
    let status = repo.status().unwrap();
    assert_eq!(status.branch.as_deref(), Some("feature"));
    assert_eq!(status.upstream.as_deref(), Some("origin/feature"));

    repo.switch("main", None).unwrap();
    assert_eq!(repo.status().unwrap().branch.as_deref(), Some("main"));
    assert_eq!(
        names(&repo.branches().unwrap())[1],
        ("feature".into(), None, false)
    );
}

#[test]
fn uncommitted_changes_travel_with_the_switch() {
    let f = Fixture::new();
    let repo = Repository::new(&f.work, &f.env);
    fs::write(f.work.join("b.txt"), "work in progress\n").unwrap();
    repo.switch("feature", Some("origin")).unwrap();
    assert_eq!(repo.status().unwrap().changes, 1);
    assert_eq!(
        fs::read_to_string(f.work.join("b.txt")).unwrap(),
        "work in progress\n"
    );
}

#[test]
fn a_switch_that_would_overwrite_changes_is_refused() {
    let f = Fixture::new();
    let repo = Repository::new(&f.work, &f.env);
    fs::write(f.work.join("a.txt"), "local edit\n").unwrap();
    let err = repo.switch("feature", Some("origin")).unwrap_err();
    assert!(
        matches!(err, GitError::Failed(ref message) if !message.is_empty()),
        "{err:?}"
    );
    assert_eq!(repo.status().unwrap().branch.as_deref(), Some("main"));
    assert_eq!(
        fs::read_to_string(f.work.join("a.txt")).unwrap(),
        "local edit\n"
    );
}

#[test]
fn names_that_look_like_options_are_rejected() {
    let f = Fixture::new();
    let err = Repository::new(&f.work, &f.env)
        .switch("-f", None)
        .unwrap_err();
    assert!(matches!(err, GitError::InvalidBranch(_)));
}

#[test]
fn fetch_brings_new_branches_and_prunes_deleted_ones() {
    let f = Fixture::new();
    f.git(&f.seed, &["switch", "-c", "fresh"]);
    f.git(&f.seed, &["push", "origin", "fresh"]);
    f.git(&f.seed, &["push", "origin", "--delete", "feature"]);

    let repo = Repository::new(&f.work, &f.env);
    repo.fetch().unwrap();
    assert_eq!(
        names(&repo.branches().unwrap()),
        vec![
            ("main".into(), None, true),
            ("fresh".into(), Some("origin".into()), false),
        ]
    );
}

#[test]
fn pull_fast_forwards_the_current_branch() {
    let f = Fixture::new();
    f.commit_in_seed("b.txt", "two\n");
    f.git(&f.seed, &["push", "origin", "main"]);

    let repo = Repository::new(&f.work, &f.env);
    repo.fetch().unwrap();
    assert_eq!(repo.status().unwrap().behind, 1);
    repo.pull().unwrap();
    assert_eq!(repo.status().unwrap().behind, 0);
    assert_eq!(fs::read_to_string(f.work.join("b.txt")).unwrap(), "two\n");
}

#[test]
fn pull_refuses_a_diverged_branch() {
    let f = Fixture::new();
    f.commit_in_seed("b.txt", "theirs\n");
    f.git(&f.seed, &["push", "origin", "main"]);
    f.commit(&f.work, "c.txt", "mine\n");

    let repo = Repository::new(&f.work, &f.env);
    let err = repo.pull().unwrap_err();
    assert!(matches!(err, GitError::Failed(_)), "{err:?}");
    let status = repo.status().unwrap();
    assert_eq!((status.ahead, status.behind), (1, 1));
}

/// Linux/macOS only: Windows also searches the parent's `PATH` and finds `git` anyway.
#[cfg(unix)]
#[test]
fn a_missing_git_is_reported_as_not_installed() {
    let f = Fixture::new();
    let mut env = f.env.clone();
    env.insert("PATH".into(), f.root.join("empty").display().to_string());
    let err = Repository::new(&f.work, &env).status().unwrap_err();
    assert!(matches!(err, GitError::NotInstalled), "{err:?}");
}
