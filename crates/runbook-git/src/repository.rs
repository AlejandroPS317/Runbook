//! The operations Runbook offers on a project's repository, one `git` invocation each.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use crate::branches::{self, Branch};
use crate::command::{Invocation, run};
use crate::error::GitError;
use crate::status::{Status, parse_status};

/// The variables Git runs with: the user's login-shell environment, so a `git` installed
/// through Homebrew or a version manager is found and its credential setup applies.
pub type Environment = BTreeMap<String, String>;

/// Safety net for fetch and pull against a remote or credential helper that never answers.
/// Inherently timed: there is no event to wait for when the other side hangs.
pub const NETWORK_TIMEOUT: Duration = Duration::from_secs(120);

/// A folder to run Git in, with the environment to run it with.
pub struct Repository<'a> {
    dir: &'a Path,
    env: &'a Environment,
}

impl<'a> Repository<'a> {
    pub fn new(dir: &'a Path, env: &'a Environment) -> Self {
        Self { dir, env }
    }

    /// Whether the folder is the top level of a Git work tree. `false` outside any
    /// repository, in a subfolder of one, and inside a `.git` folder or a bare repository.
    pub fn is_top_level(&self) -> Result<bool, GitError> {
        // Prints `true` and then the folder's path relative to the top level, empty there.
        match self.git(&["rev-parse", "--is-inside-work-tree", "--show-prefix"]) {
            Ok(output) => {
                let mut lines = output.lines();
                Ok(lines.next() == Some("true") && lines.next().unwrap_or("").is_empty())
            }
            Err(GitError::Failed(_)) => Ok(false),
            Err(err) => Err(err),
        }
    }

    pub fn status(&self) -> Result<Status, GitError> {
        // Without optional locks, a refresh never takes `index.lock` and so never makes
        // the user's own `git` commands fail while it runs.
        let output = run(&Invocation {
            dir: self.dir,
            env: self.env,
            args: &["status", "--porcelain=v2", "--branch"],
            extra_env: &[("GIT_OPTIONAL_LOCKS", "0")],
            timeout: None,
        })?;
        parse_status(&output)
    }

    /// Most recently committed first; see [`Branch`] for the grouping.
    pub fn branches(&self) -> Result<Vec<Branch>, GitError> {
        let output = self.git(&[
            "for-each-ref",
            "--sort=-committerdate",
            branches::FORMAT,
            "refs/heads",
            "refs/remotes",
        ])?;
        Ok(branches::parse_branches(&output))
    }

    /// Switches to a local branch, or creates a local branch tracking `remote`'s branch of
    /// that name. Uncommitted changes come along when Git can carry them; otherwise Git
    /// refuses and nothing changes.
    pub fn switch(&self, name: &str, remote: Option<&str>) -> Result<(), GitError> {
        validate(name)?;
        match remote {
            None => self.git(&["switch", name]),
            Some(remote) => {
                validate(remote)?;
                // Explicit `--track`: Git's own guess fails when several remotes have a
                // branch of that name.
                self.git(&["switch", "--track", &format!("{remote}/{name}")])
            }
        }
        .map(drop)
    }

    /// Fetches every remote and prunes remote branches that were deleted there.
    pub fn fetch(&self) -> Result<(), GitError> {
        self.network(&["fetch", "--all", "--prune"])
    }

    /// Fast-forwards the current branch to its upstream; fails if they diverged.
    pub fn pull(&self) -> Result<(), GitError> {
        self.network(&["pull", "--ff-only"])
    }

    fn git(&self, args: &[&str]) -> Result<String, GitError> {
        run(&Invocation {
            dir: self.dir,
            env: self.env,
            args,
            extra_env: &[],
            timeout: None,
        })
    }

    fn network(&self, args: &[&str]) -> Result<(), GitError> {
        run(&Invocation {
            dir: self.dir,
            env: self.env,
            args,
            extra_env: &[],
            timeout: Some(NETWORK_TIMEOUT),
        })
        .map(drop)
    }
}

/// Names come from Git's own listing, but are checked anyway so that one starting with `-`
/// can never be read as an option.
fn validate(name: &str) -> Result<(), GitError> {
    if name.is_empty() || name.starts_with('-') {
        return Err(GitError::InvalidBranch(name.to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_names_that_look_like_options() {
        assert!(matches!(validate("-f"), Err(GitError::InvalidBranch(_))));
        assert!(matches!(validate(""), Err(GitError::InvalidBranch(_))));
        assert!(validate("feat/-x").is_ok());
    }
}
