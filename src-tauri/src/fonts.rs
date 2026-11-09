//! Linux workaround for the WebKitGTK hang caused by WOFF fonts (ADR 0006).
//!
//! Linux: WebKitGTK 2.52 never paints when fontconfig exposes some WOFF fonts (OpenDyslexic
//! from `fonts-opendyslexic`, preinstalled on Zorin OS). Runbook points `FONTCONFIG_FILE`
//! at a file that loads the system configuration and rejects WOFF/WOFF2 files. The globs
//! start with `/` (`/*.woff`): fontconfig only applies globs to absolute font paths, and
//! `*.woff` or `*/*.woff` reject nothing (checked with `fc-list`).
//! macOS/Windows: WebKit there does not use fontconfig; both functions do nothing.

use std::collections::BTreeMap;

#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
const SYSTEM_CONFIG: &str = "/etc/fonts/fonts.conf";

#[cfg(target_os = "linux")]
fn config_contents() -> String {
    format!(
        r#"<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<!-- Written by Runbook (ADR 0006): WebKitGTK hangs on some WOFF fonts. -->
<fontconfig>
  <include ignore_missing="yes">{SYSTEM_CONFIG}</include>
  <selectfont>
    <rejectfont>
      <glob>/*.woff</glob>
      <glob>/*.woff2</glob>
    </rejectfont>
  </selectfont>
</fontconfig>
"#
    )
}

/// `$XDG_CACHE_HOME/runbook/fontconfig.conf`, or `~/.cache/runbook/fontconfig.conf`.
#[cfg(target_os = "linux")]
fn config_path() -> Option<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(cache.join("runbook").join("fontconfig.conf"))
}

/// Points Runbook's own fontconfig at the WOFF-free configuration.
///
/// Must run at the very start of the process, before any thread exists: it changes the
/// process environment, which is not thread-safe.
pub fn apply_workaround() {
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("FONTCONFIG_FILE").is_some()
            || !std::path::Path::new(SYSTEM_CONFIG).is_file()
        {
            return;
        }
        let Some(path) = config_path() else { return };
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, config_contents()));
        if written.is_ok() {
            // SAFETY: called first thing in `run`, before Tauri, GTK or any other thread
            // starts, so nothing else reads or writes the environment concurrently.
            unsafe { std::env::set_var("FONTCONFIG_FILE", &path) };
        }
    }
}

/// Removes the workaround from a script's environment: scripts should see the user's
/// normal font configuration.
pub fn scrub(env: &mut BTreeMap<String, String>) {
    #[cfg(target_os = "linux")]
    {
        let ours = config_path().map(|p| p.display().to_string());
        if ours.is_some() && env.get("FONTCONFIG_FILE") == ours.as_ref() {
            env.remove("FONTCONFIG_FILE");
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = env;
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn config_includes_the_system_file_and_rejects_woff() {
        let text = config_contents();
        assert!(text.contains(r#"<include ignore_missing="yes">/etc/fonts/fonts.conf</include>"#));
        assert!(text.contains("<glob>/*.woff</glob>"));
        assert!(text.contains("<glob>/*.woff2</glob>"));
    }

    #[test]
    fn scrub_removes_only_runbooks_own_override() {
        let ours = config_path().unwrap().display().to_string();
        let mut env = BTreeMap::from([("FONTCONFIG_FILE".to_owned(), ours)]);
        scrub(&mut env);
        assert!(!env.contains_key("FONTCONFIG_FILE"));

        let mut env = BTreeMap::from([("FONTCONFIG_FILE".to_owned(), "/custom.conf".to_owned())]);
        scrub(&mut env);
        assert_eq!(
            env.get("FONTCONFIG_FILE").map(String::as_str),
            Some("/custom.conf")
        );
    }
}
