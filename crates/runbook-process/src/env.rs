//! The environment scripts run with.
//!
//! Linux and macOS: apps launched from the desktop do not inherit the variables that the
//! user's shell startup files set (nvm, fnm, volta and corepack all edit `PATH` there), so
//! `pnpm` and `node` would not be found. We start the user's login shell once, in
//! interactive mode so `.bashrc`/`.zshrc` run too, print its environment between markers,
//! and use that for every script.
//!
//! Windows: GUI apps receive the full user environment from the registry, so the current
//! process environment is used as is.

use std::collections::BTreeMap;

pub type Environment = BTreeMap<String, String>;

/// The environment of the user's login shell, or of this process when it cannot be read.
pub fn login_environment() -> Environment {
    #[cfg(unix)]
    {
        if let Some(env) = unix::read_login_shell_env() {
            return env;
        }
    }
    std::env::vars().collect()
}

#[cfg_attr(not(unix), allow(dead_code))]
const BEGIN_MARKER: &str = "__RUNBOOK_ENV_BEGIN__";
#[cfg_attr(not(unix), allow(dead_code))]
const END_MARKER: &str = "__RUNBOOK_ENV_END__";

/// Variables that describe the probing shell itself, not the user's environment.
#[cfg_attr(not(unix), allow(dead_code))]
const SHELL_LOCAL_VARS: &[&str] = &["_", "SHLVL", "PWD", "OLDPWD"];

/// Extracts `NUL`-separated `KEY=value` entries printed between the markers. Shell startup
/// files may print anything before or after (banners, warnings), which is ignored.
#[cfg_attr(not(unix), allow(dead_code))]
fn parse_marked_env(output: &[u8]) -> Option<Environment> {
    let text = String::from_utf8_lossy(output);
    let start = text.find(BEGIN_MARKER)? + BEGIN_MARKER.len();
    let end = start + text[start..].find(END_MARKER)?;
    let env: Environment = text[start..end]
        .split('\0')
        .filter_map(|entry| {
            let entry = entry.trim_start_matches('\n');
            let (key, value) = entry.split_once('=')?;
            (!key.is_empty() && !SHELL_LOCAL_VARS.contains(&key))
                .then(|| (key.to_owned(), value.to_owned()))
        })
        .collect();
    (!env.is_empty()).then_some(env)
}

#[cfg(unix)]
mod unix {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::{BEGIN_MARKER, END_MARKER, Environment, parse_marked_env};

    /// Slow startup files (nvm takes ~0.5 s) are fine; a hung one must not block forever.
    const TIMEOUT: Duration = Duration::from_secs(10);

    pub fn read_login_shell_env() -> Option<Environment> {
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "/bin/sh".to_owned());
        // `printf` and `env -0` work in sh, bash, zsh and fish. `-l -i`: login and
        // interactive, so both profile and rc files run.
        let script = format!("printf '{BEGIN_MARKER}'; env -0; printf '{END_MARKER}'");
        let mut child = Command::new(&shell)
            .args(["-l", "-i", "-c", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut stdout = child.stdout.take()?;

        // The helper thread reads until the end marker rather than until EOF: a startup
        // file may leave a background process holding stdout open. The main thread waits
        // on the channel with a deadline instead of polling.
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut output = Vec::new();
            let mut chunk = [0u8; 8192];
            while let Ok(n) = stdout.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                output.extend_from_slice(&chunk[..n]);
                if output
                    .windows(END_MARKER.len())
                    .any(|w| w == END_MARKER.as_bytes())
                {
                    break;
                }
            }
            let _ = tx.send(output);
        });
        let output = rx.recv_timeout(TIMEOUT).ok();
        let _ = child.kill();
        let _ = child.wait();
        parse_marked_env(&output?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_between_markers_and_ignores_noise() {
        let output = b"Welcome banner\n__RUNBOOK_ENV_BEGIN__PATH=/a:/b\0HOME=/home/u\0MULTI=line1\nline2\0_=/usr/bin/env\0SHLVL=2\0__RUNBOOK_ENV_END__trailing";
        let env = parse_marked_env(output).unwrap();
        assert_eq!(env.get("PATH").map(String::as_str), Some("/a:/b"));
        assert_eq!(env.get("HOME").map(String::as_str), Some("/home/u"));
        assert_eq!(env.get("MULTI").map(String::as_str), Some("line1\nline2"));
        assert!(!env.contains_key("_"));
        assert!(!env.contains_key("SHLVL"));
    }

    #[test]
    fn value_may_contain_equals_signs() {
        let env =
            parse_marked_env(b"__RUNBOOK_ENV_BEGIN__OPTS=a=b=c\0__RUNBOOK_ENV_END__").unwrap();
        assert_eq!(env.get("OPTS").map(String::as_str), Some("a=b=c"));
    }

    #[test]
    fn missing_markers_or_empty_env_is_none() {
        assert_eq!(parse_marked_env(b"PATH=/bin\0"), None);
        assert_eq!(parse_marked_env(b"__RUNBOOK_ENV_BEGIN__"), None);
        assert_eq!(
            parse_marked_env(b"__RUNBOOK_ENV_BEGIN____RUNBOOK_ENV_END__"),
            None
        );
    }

    #[test]
    fn login_environment_has_a_path() {
        // Windows spells it `Path`; variable names are case-insensitive there.
        assert!(
            login_environment()
                .keys()
                .any(|key| key.eq_ignore_ascii_case("PATH"))
        );
    }
}
