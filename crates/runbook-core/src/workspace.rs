//! Workspace definitions: which folders a monorepo root declares as its packages.
//!
//! pnpm reads `packages` from `pnpm-workspace.yaml`; npm reads `workspaces` from the root
//! `package.json` (parsed in `manifest.rs`). Both are lists of globs relative to the root,
//! where a leading `!` excludes matches.

use std::collections::HashSet;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use yaml_rust2::{Yaml, YamlLoader};

pub const PNPM_WORKSPACE_FILE: &str = "pnpm-workspace.yaml";

/// Where the root's workspace patterns came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSource {
    PnpmWorkspaceYaml,
    PackageJson,
}

/// The workspace patterns declared by the project root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspaces {
    pub source: WorkspaceSource,
    /// Patterns as written, e.g. `apps/*` or `!**/test/**`.
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkspaceIssue {
    #[error("invalid YAML: {0}")]
    InvalidYaml(String),
    #[error("\"packages\" must be a list of strings")]
    PackagesNotAList,
    #[error("invalid workspace pattern \"{pattern}\": {message}")]
    InvalidPattern { pattern: String, message: String },
}

/// Reads the `packages` list of a `pnpm-workspace.yaml`. A file without `packages` (for
/// example one that only declares a `catalog`) yields an empty list.
pub fn parse_pnpm_workspace(text: &str) -> Result<Vec<String>, WorkspaceIssue> {
    let docs = YamlLoader::load_from_str(text)
        .map_err(|err| WorkspaceIssue::InvalidYaml(err.to_string()))?;
    let Some(doc) = docs.first() else {
        return Ok(Vec::new());
    };
    match &doc["packages"] {
        Yaml::BadValue | Yaml::Null => Ok(Vec::new()),
        Yaml::Array(items) => items
            .iter()
            .map(|item| item.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or(WorkspaceIssue::PackagesNotAList),
        _ => Err(WorkspaceIssue::PackagesNotAList),
    }
}

/// Compiled workspace patterns.
#[derive(Debug)]
pub struct WorkspaceMatcher {
    include: GlobSet,
    exclude: GlobSet,
    explicit_dirs: HashSet<String>,
}

impl WorkspaceMatcher {
    /// Compiles `patterns`. Invalid patterns are skipped and returned as issues.
    pub fn new(patterns: &[String]) -> (Self, Vec<WorkspaceIssue>) {
        let mut include = GlobSetBuilder::new();
        let mut exclude = GlobSetBuilder::new();
        let mut explicit_dirs = HashSet::new();
        let mut issues = Vec::new();

        for raw in patterns {
            let (negated, pattern) = match raw.strip_prefix('!') {
                Some(rest) => (true, normalize(rest)),
                None => (false, normalize(raw)),
            };
            if pattern.is_empty() {
                continue;
            }
            // `literal_separator`: `*` stays within one folder, as in pnpm and npm.
            match GlobBuilder::new(pattern).literal_separator(true).build() {
                Ok(glob) if negated => {
                    exclude.add(glob);
                }
                Ok(glob) => {
                    include.add(glob);
                    explicit_dirs.extend(ancestors_of(literal_base(pattern)));
                }
                Err(err) => issues.push(WorkspaceIssue::InvalidPattern {
                    pattern: raw.clone(),
                    message: err.kind().to_string(),
                }),
            }
        }

        let build = |builder: GlobSetBuilder| builder.build().unwrap_or_else(|_| GlobSet::empty());
        let matcher = Self {
            include: build(include),
            exclude: build(exclude),
            explicit_dirs,
        };
        (matcher, issues)
    }

    /// Whether the package at `relative_dir` (`/`-separated, relative to the root) is a
    /// workspace member. The root itself is never a member; it is the workspace root.
    pub fn is_member(&self, relative_dir: &str) -> bool {
        !relative_dir.is_empty()
            && self.include.is_match(relative_dir)
            && !self.exclude.is_match(relative_dir)
    }

    /// Folders that patterns name literally, plus their ancestors. `tools/build` yields
    /// `tools` and `tools/build`; `packages/*` yields `packages`. Discovery searches these
    /// even when their name is normally skipped (such as `build`).
    pub fn explicit_dirs(&self) -> &HashSet<String> {
        &self.explicit_dirs
    }
}

fn normalize(pattern: &str) -> &str {
    let pattern = pattern.trim();
    let pattern = pattern.strip_prefix("./").unwrap_or(pattern);
    pattern.trim_end_matches('/')
}

/// The leading components of `pattern` that contain no glob syntax.
fn literal_base(pattern: &str) -> String {
    pattern
        .split('/')
        .take_while(|part| !part.contains(['*', '?', '[', '{']))
        .collect::<Vec<_>>()
        .join("/")
}

fn ancestors_of(dir: String) -> Vec<String> {
    let parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    (1..=parts.len()).map(|n| parts[..n].join("/")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher(patterns: &[&str]) -> WorkspaceMatcher {
        let patterns: Vec<String> = patterns.iter().map(|p| p.to_string()).collect();
        let (matcher, issues) = WorkspaceMatcher::new(&patterns);
        assert!(issues.is_empty(), "{issues:?}");
        matcher
    }

    #[test]
    fn reads_packages_and_ignores_catalogs_and_comments() {
        let yaml = r#"
packages:
  - "apps/*"
  - 'packages/*'
  # a comment
  - tools/build
catalog:
  "@scope/pkg": ^1.0.0
catalogs:
  ui:
    react: ^19.0.0
"#;
        assert_eq!(
            parse_pnpm_workspace(yaml).unwrap(),
            ["apps/*", "packages/*", "tools/build"]
        );
    }

    #[test]
    fn reads_flow_sequence() {
        assert_eq!(
            parse_pnpm_workspace("packages: [apps/*, packages/*]").unwrap(),
            ["apps/*", "packages/*"]
        );
    }

    #[test]
    fn missing_packages_key_or_empty_file_is_empty() {
        assert!(
            parse_pnpm_workspace("catalog:\n  react: ^19\n")
                .unwrap()
                .is_empty()
        );
        assert!(parse_pnpm_workspace("").unwrap().is_empty());
    }

    #[test]
    fn rejects_invalid_yaml_and_wrong_types() {
        assert!(matches!(
            parse_pnpm_workspace("packages: [apps/*").unwrap_err(),
            WorkspaceIssue::InvalidYaml(_)
        ));
        assert_eq!(
            parse_pnpm_workspace("packages: apps/*").unwrap_err(),
            WorkspaceIssue::PackagesNotAList
        );
        assert_eq!(
            parse_pnpm_workspace("packages:\n  - apps/*\n  - {a: b}\n").unwrap_err(),
            WorkspaceIssue::PackagesNotAList
        );
    }

    #[test]
    fn star_matches_one_level_only() {
        let m = matcher(&["apps/*"]);
        assert!(m.is_member("apps/web"));
        assert!(!m.is_member("apps/web/nested"));
        assert!(!m.is_member("apps"));
        assert!(!m.is_member("packages/ui"));
    }

    #[test]
    fn double_star_matches_nested_folders() {
        let m = matcher(&["packages/**"]);
        assert!(m.is_member("packages/ui"));
        assert!(m.is_member("packages/group/ui"));
    }

    #[test]
    fn negated_patterns_exclude() {
        let m = matcher(&["packages/*", "!packages/legacy", "!**/test/**"]);
        assert!(m.is_member("packages/ui"));
        assert!(!m.is_member("packages/legacy"));
        let m = matcher(&["packages/**", "!**/test/**"]);
        assert!(!m.is_member("packages/ui/test/fixture"));
    }

    #[test]
    fn normalizes_dot_slash_and_trailing_slash() {
        let m = matcher(&["./packages/*/", "./apps/web"]);
        assert!(m.is_member("packages/ui"));
        assert!(m.is_member("apps/web"));
    }

    #[test]
    fn root_is_never_a_member() {
        assert!(!matcher(&["**"]).is_member(""));
    }

    #[test]
    fn explicit_dirs_cover_literal_bases_and_ancestors() {
        let m = matcher(&["tools/build", "packages/*", "!build"]);
        let mut dirs: Vec<_> = m.explicit_dirs().iter().cloned().collect();
        dirs.sort();
        assert_eq!(dirs, ["packages", "tools", "tools/build"]);
    }

    #[test]
    fn invalid_pattern_is_reported_and_skipped() {
        let (m, issues) = WorkspaceMatcher::new(&["packages/[".into(), "apps/*".into()]);
        assert_eq!(issues.len(), 1);
        assert!(
            matches!(&issues[0], WorkspaceIssue::InvalidPattern { pattern, .. } if pattern == "packages/[")
        );
        assert!(m.is_member("apps/web"));
    }
}
