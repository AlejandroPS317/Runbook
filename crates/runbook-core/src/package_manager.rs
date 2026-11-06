//! Which package manager runs a project's scripts. Runbook v1 supports pnpm and npm.

use std::fmt;
use std::path::Path;

use crate::workspace::PNPM_WORKSPACE_FILE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Pnpm,
    Npm,
}

impl PackageManager {
    /// The executable name, e.g. for `pnpm run <script>`.
    pub fn command(self) -> &'static str {
        match self {
            Self::Pnpm => "pnpm",
            Self::Npm => "npm",
        }
    }
}

impl fmt::Display for PackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.command())
    }
}

/// The detected manager, and the `packageManager` value that was ignored because v1 does
/// not support it (e.g. `yarn`, `bun`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    pub manager: PackageManager,
    pub unsupported: Option<String>,
}

/// Detects the package manager of the project at `root`.
///
/// Order: the root `packageManager` field (`pnpm@9.15.4`), then pnpm files
/// (`pnpm-workspace.yaml`, `pnpm-lock.yaml`), then npm as the default.
pub fn detect(root: &Path, package_manager_field: Option<&str>) -> Detection {
    let declared = package_manager_field.map(|value| {
        let name = value.split('@').next().unwrap_or_default().trim();
        match name {
            "pnpm" => Ok(PackageManager::Pnpm),
            "npm" => Ok(PackageManager::Npm),
            other => Err(other.to_owned()),
        }
    });
    match declared {
        Some(Ok(manager)) => Detection {
            manager,
            unsupported: None,
        },
        Some(Err(name)) => Detection {
            manager: detect_from_files(root),
            unsupported: Some(name),
        },
        None => Detection {
            manager: detect_from_files(root),
            unsupported: None,
        },
    }
}

fn detect_from_files(root: &Path) -> PackageManager {
    let has = |file: &str| root.join(file).is_file();
    if has(PNPM_WORKSPACE_FILE) || has("pnpm-lock.yaml") {
        PackageManager::Pnpm
    } else {
        PackageManager::Npm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn detect_in(files: &[&str], field: Option<&str>) -> Detection {
        let tmp = tempfile::tempdir().unwrap();
        for file in files {
            fs::write(tmp.path().join(file), "").unwrap();
        }
        detect(tmp.path(), field)
    }

    #[test]
    fn package_manager_field_wins() {
        let d = detect_in(&["pnpm-lock.yaml"], Some("npm@10.9.0"));
        assert_eq!(d.manager, PackageManager::Npm);
        assert_eq!(d.unsupported, None);
        let d = detect_in(&[], Some("pnpm@9.15.4+sha512.abc"));
        assert_eq!(d.manager, PackageManager::Pnpm);
    }

    #[test]
    fn pnpm_files_mean_pnpm() {
        assert_eq!(
            detect_in(&["pnpm-workspace.yaml"], None).manager,
            PackageManager::Pnpm
        );
        assert_eq!(
            detect_in(&["pnpm-lock.yaml"], None).manager,
            PackageManager::Pnpm
        );
    }

    #[test]
    fn defaults_to_npm() {
        assert_eq!(
            detect_in(&["package-lock.json"], None).manager,
            PackageManager::Npm
        );
        assert_eq!(detect_in(&[], None).manager, PackageManager::Npm);
    }

    #[test]
    fn unsupported_manager_falls_back_and_is_reported() {
        let d = detect_in(&["pnpm-lock.yaml"], Some("yarn@4.5.0"));
        assert_eq!(d.manager, PackageManager::Pnpm);
        assert_eq!(d.unsupported.as_deref(), Some("yarn"));
    }
}
