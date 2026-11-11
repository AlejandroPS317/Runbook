# 0008 – Git through the user's `git` command

- Status: Accepted
- Date: 2026-11-11

## Context

M9 adds basic branch control: show the current branch, switch, fetch and pull. Fetch and
pull talk to remotes, which needs the user's credentials: SSH keys and agents, HTTPS
credential helpers (Git Credential Manager, libsecret, the macOS keychain), `insteadOf`
rewrites, proxies and hooks. The user already has all of that configured for their own
`git`. Runbook competes with IDEs on size and RAM, and its stack is listed in
[ADR 0001](0001-tech-stack.md).

## Decision

Runbook runs the user's `git` executable, found on the login-shell `PATH`, from a new
crate `runbook-git`. It never prompts: stdin is null, `GIT_TERMINAL_PROMPT=0`, and on Linux
and macOS `git` starts in a new session without a controlling terminal. Only porcelain
output (`status --porcelain=v2`, `for-each-ref --format`) is parsed; everything else is
shown to the user as Git prints it. Without `git`, the feature hides itself.

## Alternatives considered

- **`git2` (libgit2 bindings)**: a C library to build on three OSes, adds binary size, and
  reimplements authentication: SSH agents, credential helpers and `insteadOf` need custom
  callbacks, and behavior drifts from the user's `git` (hooks, config, new features).
- **`gix` (gitoxide)**: pure Rust, but a large dependency tree for five operations, and
  any gap with the user's `git` (hooks, config, transports) shows up as "works in my
  terminal, not in Runbook".
- **`gh` (GitHub CLI)**: only covers GitHub, and branch operations still go through `git`.

## Consequences

- Zero new dependencies and no extra binary size; credentials keep working exactly as in
  the user's terminal, and Runbook never stores any.
- A runtime requirement on `git`, which developers already have; the feature is optional.
- Each operation spawns a process (a few milliseconds locally), so status refreshes are
  event-driven, never periodic.
- Output parsing is limited to porcelain formats, which Git keeps stable across versions
  and locales.
