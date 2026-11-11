//! Temporary repositories for the integration tests: a bare `remote.git`, a `seed` clone
//! that plays "someone else pushing", and a `work` clone that Runbook acts on. Git's
//! global and system configuration are replaced so the machine's setup does not matter.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use runbook_git::Environment;
use tempfile::TempDir;

/// Variables of the test runner that would make Git prompt, use a proxy or read a config.
const DROPPED: &[&str] = &[
    "GIT_ASKPASS",
    "SSH_ASKPASS",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
];

pub struct Fixture {
    // Removed on drop; kept to own the folder.
    _dir: TempDir,
    pub root: PathBuf,
    pub seed: PathBuf,
    pub work: PathBuf,
    pub env: Environment,
}

impl Fixture {
    /// `main` with `a.txt` and `b.txt`, plus a pushed `feature` branch that changes `a.txt`.
    /// `work` is a fresh clone: `main` locally, `feature` only on `origin`.
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("temporary folder");
        let root = dir.path().to_path_buf();
        let env = environment(&root);
        let fixture = Fixture {
            seed: root.join("seed"),
            work: root.join("work"),
            root,
            env,
            _dir: dir,
        };
        fixture.git(
            &fixture.root,
            &["init", "--bare", "-b", "main", "remote.git"],
        );
        fixture.git(&fixture.root, &["clone", "remote.git", "seed"]);
        fixture.commit_in_seed("a.txt", "one\n");
        fixture.commit_in_seed("b.txt", "one\n");
        fixture.git(&fixture.seed, &["push", "origin", "main"]);
        fixture.git(&fixture.seed, &["switch", "-c", "feature"]);
        fixture.commit_in_seed("a.txt", "feature\n");
        fixture.git(&fixture.seed, &["push", "origin", "feature"]);
        fixture.git(&fixture.seed, &["switch", "main"]);
        fixture.git(&fixture.root, &["clone", "remote.git", "work"]);
        fixture
    }

    /// Runs a setup command; panics with Git's message when it fails.
    pub fn git(&self, dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env_clear()
            .envs(&self.env)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    pub fn commit_in_seed(&self, file: &str, contents: &str) {
        self.commit(&self.seed, file, contents);
    }

    pub fn commit(&self, dir: &Path, file: &str, contents: &str) {
        fs::write(dir.join(file), contents).expect("write file");
        self.git(dir, &["add", file]);
        self.git(dir, &["commit", "-m", &format!("change {file}")]);
    }
}

/// The test process environment with an isolated Git configuration.
fn environment(root: &Path) -> Environment {
    let config = root.join("gitconfig");
    fs::write(
        &config,
        "[user]\n\tname = Runbook Test\n\temail = test@example.com\n\
         [init]\n\tdefaultBranch = main\n\
         [commit]\n\tgpgsign = false\n\
         [advice]\n\tdetachedHead = false\n",
    )
    .expect("write gitconfig");
    let mut env: Environment = std::env::vars()
        .filter(|(key, _)| !DROPPED.contains(&key.as_str()))
        .collect();
    env.insert("GIT_CONFIG_GLOBAL".into(), config.display().to_string());
    env.insert("GIT_CONFIG_NOSYSTEM".into(), "1".into());
    // Git must not find a repository above the temporary folder.
    env.insert("GIT_CEILING_DIRECTORIES".into(), root.display().to_string());
    env
}
