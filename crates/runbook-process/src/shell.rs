//! The user's interactive shell, opened by a project terminal tab, and the command lines
//! Runbook types into it for the user to edit and run.

use crate::env::Environment;

/// How to start the user's interactive shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    pub program: String,
    pub args: Vec<String>,
}

/// The shell to open in a terminal tab.
///
/// Linux and macOS: `$SHELL` from the captured environment, or `/bin/sh` when it is unset.
/// It starts without `-c`, so with a terminal on stdin it is interactive and reads its rc
/// file (prompt, aliases); it is not a login shell because the environment already comes
/// from one (see `env.rs`).
///
/// Windows: Windows PowerShell, which ships with every supported Windows version; `-NoLogo`
/// hides the copyright banner.
pub fn user_shell(env: &Environment) -> ShellCommand {
    #[cfg(unix)]
    {
        let program = env
            .get("SHELL")
            .filter(|shell| !shell.is_empty())
            .cloned()
            .unwrap_or_else(|| "/bin/sh".to_owned());
        ShellCommand {
            program,
            args: Vec::new(),
        }
    }
    #[cfg(windows)]
    {
        let _ = env;
        ShellCommand {
            program: "powershell.exe".to_owned(),
            args: vec!["-NoLogo".to_owned()],
        }
    }
}

/// Joins `args` into one line for the user's shell, quoting only what needs it.
///
/// The line is typed at the prompt and run by the user, never by Runbook, so it is the one
/// place where a command becomes a string. Linux and macOS: POSIX single quotes, which
/// bash, zsh and fish read the same way unless the text contains a backslash. Windows:
/// PowerShell single quotes, where a quote is escaped by doubling it.
pub fn command_line(args: &[&str]) -> String {
    args.iter()
        .map(|arg| quote(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Characters no supported shell treats specially anywhere in a word. `=`, `@`, `~` and
/// `%` are left out: zsh, PowerShell and fish expand them at the start of a word.
fn is_plain(arg: &str) -> bool {
    !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':' | '+'))
}

fn quote(arg: &str) -> String {
    if is_plain(arg) {
        return arg.to_owned();
    }
    #[cfg(unix)]
    {
        format!("'{}'", arg.replace('\'', r"'\''"))
    }
    #[cfg(windows)]
    {
        format!("'{}'", arg.replace('\'', "''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_words_are_not_quoted() {
        assert_eq!(
            command_line(&["pnpm", "run", "db:migrate"]),
            "pnpm run db:migrate"
        );
        assert_eq!(
            command_line(&["npm", "run", "build.prod-v2"]),
            "npm run build.prod-v2"
        );
    }

    #[test]
    fn words_with_special_characters_are_quoted() {
        assert_eq!(command_line(&["pnpm", "run", "a b"]), "pnpm run 'a b'");
        assert_eq!(command_line(&["pnpm", "run", "x;rm"]), "pnpm run 'x;rm'");
        assert_eq!(command_line(&["pnpm", "run", "=x"]), "pnpm run '=x'");
        assert_eq!(command_line(&["pnpm", "run", ""]), "pnpm run ''");
    }

    #[cfg(unix)]
    #[test]
    fn single_quotes_are_escaped_for_posix_shells() {
        assert_eq!(command_line(&["it's"]), r"'it'\''s'");
    }

    #[cfg(windows)]
    #[test]
    fn single_quotes_are_doubled_for_powershell() {
        assert_eq!(command_line(&["it's"]), "'it''s'");
    }

    #[cfg(unix)]
    #[test]
    fn shell_comes_from_the_environment() {
        let env = Environment::from([("SHELL".to_owned(), "/usr/bin/zsh".to_owned())]);
        assert_eq!(user_shell(&env).program, "/usr/bin/zsh");
        assert!(user_shell(&env).args.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn missing_or_empty_shell_falls_back_to_sh() {
        assert_eq!(user_shell(&Environment::new()).program, "/bin/sh");
        let env = Environment::from([("SHELL".to_owned(), String::new())]);
        assert_eq!(user_shell(&env).program, "/bin/sh");
    }
}
