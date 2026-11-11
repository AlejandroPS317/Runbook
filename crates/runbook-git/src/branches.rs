//! Local and remote branches, from `git for-each-ref`.

/// Fields separated by NUL, which ref names cannot contain.
pub(crate) const FORMAT: &str = "--format=%(HEAD)%00%(refname)%00%(symref)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    /// Branch name without the remote, for example `feat/login`.
    pub name: String,
    /// The remote it lives on, for remote branches without a local branch of that name.
    pub remote: Option<String>,
    /// Whether `HEAD` is on it. Only ever true for a local branch.
    pub current: bool,
}

/// Local branches (the current one first), then remote branches that have no local branch
/// of the same name. Each group keeps Git's order. `<remote>/HEAD` is skipped.
///
/// The remote is the first path segment after `refs/remotes/`; a remote whose own name
/// contains `/` would be split wrongly, which only affects the label.
pub(crate) fn parse_branches(output: &str) -> Vec<Branch> {
    let mut local = Vec::new();
    let mut remote = Vec::new();
    for line in output.lines() {
        let mut fields = line.split('\0');
        let (Some(head), Some(refname), Some(symref)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if !symref.is_empty() {
            continue;
        }
        if let Some(name) = refname.strip_prefix("refs/heads/") {
            local.push(Branch {
                name: name.to_owned(),
                remote: None,
                current: head == "*",
            });
        } else if let Some((remote_name, name)) = refname
            .strip_prefix("refs/remotes/")
            .and_then(|rest| rest.split_once('/'))
        {
            remote.push(Branch {
                name: name.to_owned(),
                remote: Some(remote_name.to_owned()),
                current: false,
            });
        }
    }
    remote.retain(|r| !local.iter().any(|l: &Branch| l.name == r.name));
    local.sort_by_key(|b| !b.current);
    local.extend(remote);
    local
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(name: &str, current: bool) -> Branch {
        Branch {
            name: name.into(),
            remote: None,
            current,
        }
    }

    fn remote(remote: &str, name: &str) -> Branch {
        Branch {
            name: name.into(),
            remote: Some(remote.into()),
            current: false,
        }
    }

    #[test]
    fn current_first_then_remote_only_branches() {
        let output = " \0refs/heads/main\0\n\
                      *\0refs/heads/feat/login\0\n\
                      \x20\0refs/remotes/origin/HEAD\0refs/remotes/origin/main\n\
                      \x20\0refs/remotes/origin/main\0\n\
                      \x20\0refs/remotes/origin/release/v1\0\n\
                      \x20\0refs/remotes/upstream/feat/login\0\n\
                      \x20\0refs/remotes/upstream/docs\0\n";
        assert_eq!(
            parse_branches(output),
            vec![
                local("feat/login", true),
                local("main", false),
                remote("origin", "release/v1"),
                remote("upstream", "docs"),
            ]
        );
    }

    #[test]
    fn detached_head_marks_nothing_current() {
        let branches = parse_branches(" \0refs/heads/main\0\n \0refs/heads/dev\0\n");
        assert_eq!(branches, vec![local("main", false), local("dev", false)]);
    }

    #[test]
    fn ignores_blank_and_unknown_lines() {
        assert!(parse_branches("\n \0refs/tags/v1\0\ngarbage\n").is_empty());
    }
}
