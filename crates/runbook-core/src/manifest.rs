//! Parsing of a single `package.json` into the fields Runbook uses.
//!
//! Parsing is tolerant: a broken `scripts` entry costs that entry, not the whole file.
//! Only text that is not a JSON object at all is rejected.

use serde_json::Value;

/// One entry of the `scripts` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub name: String,
    pub command: String,
}

/// The parts of a `package.json` that Runbook uses.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    /// The `name` field, when it is a string.
    pub name: Option<String>,
    /// Scripts in the order they appear in the file.
    pub scripts: Vec<Script>,
    /// The `packageManager` field (e.g. `pnpm@9.15.4`), when it is a string.
    pub package_manager: Option<String>,
    /// npm workspace patterns, from `"workspaces": [...]` or `"workspaces": {"packages": [...]}`.
    pub workspaces: Option<Vec<String>>,
}

/// A defect found while reading a `package.json`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestIssue {
    #[error("invalid JSON: {0}")]
    InvalidJson(String),
    #[error("the top-level value is not a JSON object")]
    NotAnObject,
    #[error("\"scripts\" is not an object; no scripts were read")]
    ScriptsNotAnObject,
    #[error("script \"{0}\" is not a string and was skipped")]
    NonStringScript(String),
    #[error("\"workspaces\" must be a list of strings or an object with a \"packages\" list")]
    InvalidWorkspaces,
}

/// A manifest that could be read, plus the non-fatal issues found in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedManifest {
    pub manifest: Manifest,
    pub issues: Vec<ManifestIssue>,
}

/// Parses the text of a `package.json`.
///
/// Returns `Err` only when the text is not a JSON object (`InvalidJson`, `NotAnObject`).
pub fn parse_manifest(text: &str) -> Result<ParsedManifest, ManifestIssue> {
    // Editors on Windows sometimes save a UTF-8 BOM; npm accepts it, serde_json does not.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let value: Value =
        serde_json::from_str(text).map_err(|err| ManifestIssue::InvalidJson(err.to_string()))?;
    let Value::Object(fields) = value else {
        return Err(ManifestIssue::NotAnObject);
    };

    let string_field = |key: &str| fields.get(key).and_then(Value::as_str).map(str::to_owned);
    let mut issues = Vec::new();
    let manifest = Manifest {
        name: string_field("name"),
        scripts: read_scripts(fields.get("scripts"), &mut issues),
        package_manager: string_field("packageManager"),
        workspaces: read_workspaces(fields.get("workspaces"), &mut issues),
    };
    Ok(ParsedManifest { manifest, issues })
}

fn read_workspaces(value: Option<&Value>, issues: &mut Vec<ManifestIssue>) -> Option<Vec<String>> {
    let list = match value? {
        Value::Array(list) => Some(list),
        // Object form, used by Yarn 1 and accepted by npm: {"packages": [...], "nohoist": [...]}.
        Value::Object(object) => object.get("packages").and_then(Value::as_array),
        _ => None,
    };
    let patterns: Option<Vec<String>> = list.and_then(|list| {
        list.iter()
            .map(|item| item.as_str().map(str::to_owned))
            .collect()
    });
    if patterns.is_none() {
        issues.push(ManifestIssue::InvalidWorkspaces);
    }
    patterns
}

fn read_scripts(value: Option<&Value>, issues: &mut Vec<ManifestIssue>) -> Vec<Script> {
    let Some(value) = value else {
        return Vec::new();
    };
    let Some(entries) = value.as_object() else {
        issues.push(ManifestIssue::ScriptsNotAnObject);
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|(name, command)| match command.as_str() {
            Some(command) => Some(Script {
                name: name.clone(),
                command: command.to_owned(),
            }),
            None => {
                issues.push(ManifestIssue::NonStringScript(name.clone()));
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script_names(parsed: &ParsedManifest) -> Vec<&str> {
        parsed
            .manifest
            .scripts
            .iter()
            .map(|s| s.name.as_str())
            .collect()
    }

    #[test]
    fn reads_name_and_scripts() {
        let parsed =
            parse_manifest(r#"{"name": "@acme/web", "scripts": {"dev": "vite"}}"#).unwrap();
        assert_eq!(parsed.manifest.name.as_deref(), Some("@acme/web"));
        assert_eq!(
            parsed.manifest.scripts,
            vec![Script {
                name: "dev".into(),
                command: "vite".into()
            }]
        );
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn keeps_scripts_in_file_order() {
        let parsed =
            parse_manifest(r#"{"scripts": {"zeta": "z", "alpha": "a", "mid": "m"}}"#).unwrap();
        assert_eq!(script_names(&parsed), ["zeta", "alpha", "mid"]);
    }

    #[test]
    fn missing_scripts_is_not_an_issue() {
        let parsed = parse_manifest(r#"{"name": "lib"}"#).unwrap();
        assert!(parsed.manifest.scripts.is_empty());
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn missing_or_non_string_name_is_none() {
        assert_eq!(parse_manifest("{}").unwrap().manifest.name, None);
        assert_eq!(
            parse_manifest(r#"{"name": 7}"#).unwrap().manifest.name,
            None
        );
    }

    #[test]
    fn accepts_utf8_bom() {
        let parsed = parse_manifest("\u{feff}{\"scripts\": {\"dev\": \"vite\"}}").unwrap();
        assert_eq!(script_names(&parsed), ["dev"]);
    }

    #[test]
    fn rejects_invalid_json() {
        let err = parse_manifest(r#"{"scripts": {"dev": "vite""#).unwrap_err();
        assert!(matches!(err, ManifestIssue::InvalidJson(_)));
    }

    #[test]
    fn rejects_empty_text() {
        assert!(matches!(
            parse_manifest("").unwrap_err(),
            ManifestIssue::InvalidJson(_)
        ));
    }

    #[test]
    fn rejects_non_object_root() {
        assert_eq!(
            parse_manifest(r#"["a", "b"]"#).unwrap_err(),
            ManifestIssue::NotAnObject
        );
    }

    #[test]
    fn scripts_not_an_object_keeps_the_manifest() {
        let parsed = parse_manifest(r#"{"name": "x", "scripts": ["dev"]}"#).unwrap();
        assert_eq!(parsed.manifest.name.as_deref(), Some("x"));
        assert!(parsed.manifest.scripts.is_empty());
        assert_eq!(parsed.issues, [ManifestIssue::ScriptsNotAnObject]);
    }

    #[test]
    fn reads_package_manager() {
        let parsed = parse_manifest(r#"{"packageManager": "pnpm@9.15.4"}"#).unwrap();
        assert_eq!(
            parsed.manifest.package_manager.as_deref(),
            Some("pnpm@9.15.4")
        );
    }

    #[test]
    fn reads_workspaces_array_form() {
        let parsed = parse_manifest(r#"{"workspaces": ["packages/*", "apps/web"]}"#).unwrap();
        assert_eq!(
            parsed.manifest.workspaces,
            Some(vec!["packages/*".into(), "apps/web".into()])
        );
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn reads_workspaces_object_form() {
        let parsed =
            parse_manifest(r#"{"workspaces": {"packages": ["packages/*"], "nohoist": ["**"]}}"#)
                .unwrap();
        assert_eq!(parsed.manifest.workspaces, Some(vec!["packages/*".into()]));
    }

    #[test]
    fn invalid_workspaces_are_reported() {
        for text in [
            r#"{"workspaces": "packages/*"}"#,
            r#"{"workspaces": ["packages/*", 3]}"#,
            r#"{"workspaces": {"nohoist": ["**"]}}"#,
        ] {
            let parsed = parse_manifest(text).unwrap();
            assert_eq!(parsed.manifest.workspaces, None, "{text}");
            assert_eq!(parsed.issues, [ManifestIssue::InvalidWorkspaces], "{text}");
        }
    }

    #[test]
    fn non_string_script_is_skipped_and_reported() {
        let parsed =
            parse_manifest(r#"{"scripts": {"ok": "echo ok", "bad": 42, "also": "x"}}"#).unwrap();
        assert_eq!(script_names(&parsed), ["ok", "also"]);
        assert_eq!(
            parsed.issues,
            [ManifestIssue::NonStringScript("bad".into())]
        );
    }
}
