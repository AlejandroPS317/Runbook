//! The branch and working-tree state, from `git status --porcelain=v2 --branch`.

use crate::error::GitError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    /// `None` in detached HEAD.
    pub branch: Option<String>,
    /// The commit `HEAD` points to; `None` before the first commit.
    pub commit: Option<String>,
    /// For example `origin/main`; `None` when the branch tracks nothing.
    pub upstream: Option<String>,
    /// Commits on the branch that the upstream lacks, as of the last fetch.
    pub ahead: u32,
    /// Commits on the upstream that the branch lacks, as of the last fetch.
    pub behind: u32,
    /// Modified, staged, unmerged and untracked files.
    pub changes: u32,
}

/// Parses porcelain v2 output: `# branch.*` headers, then one line per changed file. The
/// format is stable across Git versions and locales; file names never span lines.
pub(crate) fn parse_status(output: &str) -> Result<Status, GitError> {
    let mut status = Status::default();
    for line in output.lines() {
        let Some(header) = line.strip_prefix("# ") else {
            if !line.is_empty() {
                status.changes += 1;
            }
            continue;
        };
        let (key, value) = header.split_once(' ').unwrap_or((header, ""));
        match key {
            "branch.oid" => status.commit = (value != "(initial)").then(|| value.to_owned()),
            "branch.head" => status.branch = (value != "(detached)").then(|| value.to_owned()),
            "branch.upstream" => status.upstream = Some(value.to_owned()),
            "branch.ab" => (status.ahead, status.behind) = parse_ahead_behind(value)?,
            _ => {}
        }
    }
    Ok(status)
}

/// `+<ahead> -<behind>`.
fn parse_ahead_behind(value: &str) -> Result<(u32, u32), GitError> {
    let invalid = || GitError::Parse(format!("branch.ab {value}"));
    let (ahead, behind) = value.split_once(' ').ok_or_else(invalid)?;
    let ahead = ahead.strip_prefix('+').ok_or_else(invalid)?;
    let behind = behind.strip_prefix('-').ok_or_else(invalid)?;
    Ok((
        ahead.parse().map_err(|_| invalid())?,
        behind.parse().map_err(|_| invalid())?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OID: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

    #[test]
    fn branch_with_upstream_and_changes() {
        let output = format!(
            "# branch.oid {OID}\n# branch.head feat/login\n# branch.upstream origin/feat/login\n# branch.ab +2 -1\n\
             1 .M N... 100644 100644 100644 {OID} {OID} src/a.ts\n\
             2 R. N... 100644 100644 100644 {OID} {OID} R100 new name.ts\told.ts\n\
             u UU N... 100644 100644 100644 100644 {OID} {OID} {OID} conflict.ts\n\
             ? untracked.txt\n"
        );
        assert_eq!(
            parse_status(&output).unwrap(),
            Status {
                branch: Some("feat/login".into()),
                commit: Some(OID.into()),
                upstream: Some("origin/feat/login".into()),
                ahead: 2,
                behind: 1,
                changes: 4,
            }
        );
    }

    #[test]
    fn detached_head_without_upstream() {
        let status =
            parse_status(&format!("# branch.oid {OID}\n# branch.head (detached)\n")).unwrap();
        assert_eq!(status.branch, None);
        assert_eq!(status.commit.as_deref(), Some(OID));
        assert_eq!(status.upstream, None);
        assert_eq!((status.ahead, status.behind, status.changes), (0, 0, 0));
    }

    #[test]
    fn repository_without_commits() {
        let status = parse_status("# branch.oid (initial)\n# branch.head main\n? a.txt\n").unwrap();
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.commit, None);
        assert_eq!(status.changes, 1);
    }

    #[test]
    fn upstream_that_no_longer_exists_has_no_counts() {
        let output =
            format!("# branch.oid {OID}\n# branch.head old\n# branch.upstream origin/old\n");
        let status = parse_status(&output).unwrap();
        assert_eq!(status.upstream.as_deref(), Some("origin/old"));
        assert_eq!((status.ahead, status.behind), (0, 0));
    }

    #[test]
    fn malformed_counts_are_an_error() {
        assert!(matches!(
            parse_status("# branch.ab 2 1\n"),
            Err(GitError::Parse(_))
        ));
        assert!(matches!(
            parse_status("# branch.ab +x -1\n"),
            Err(GitError::Parse(_))
        ));
    }
}
