# AGENTS.md

Rules for every agent working on Runbook (Claude Code, Cursor, or any other).
This file is the single source of truth for project rules; `CLAUDE.md` only imports it.

## What Runbook is

A lightweight, open source desktop app to run and manage `package.json` scripts in
JavaScript projects and monorepos. Rust core, Tauri 2 shell, Svelte 5 frontend,
xterm.js terminals backed by real PTYs. Scope, milestones and per-platform status
live in [`docs/ROADMAP.md`](docs/ROADMAP.md).

## Language

- Talk to the user in **Spanish**.
- Write everything in the repository in **English**: code, identifiers, comments,
  commit messages, PRs, docs, ADRs.

## Platform priority: Linux first

Linux is the primary platform ([ADR 0002](docs/decisions/0002-linux-first.md)).
Implement and accept every milestone on Linux first. Platform-independent code
(discovery, parsing, workspaces) must pass CI on Linux, macOS and Windows from day one.
Platform-specific code (PTY, process termination, packaging) ships for Linux first and
is tracked per platform in the roadmap.

## Repository layout

```
Cargo.toml              Cargo workspace (members: crates/*, src-tauri)
package.json            Frontend (Svelte 5 + Vite + TypeScript), managed with pnpm
src/                    Svelte frontend
src/lib/ipc/            The only place that calls Tauri `invoke`/channels; typed wrappers
src-tauri/              Tauri shell: commands, app state, wiring. Thin, no business logic
crates/runbook-core/    Discovery, package.json parsing, workspaces. Pure Rust, no Tauri
crates/runbook-process/ PTY sessions and process-tree termination (from M3)
fixtures/               Sample projects and monorepos used by tests
docs/ROADMAP.md         Milestones, acceptance criteria, per-platform status
docs/decisions/         Architecture Decision Records (ADRs)
```

Dependency direction: `src-tauri` → `crates/*`. Crates never depend on Tauri.
`runbook-core` never depends on `runbook-process`. Logic that can be tested without a
window belongs in a crate, not in `src-tauri`.

## Workflow

- **One change per branch.** Work in a dedicated branch, ideally a git worktree:
  `git worktree add ../Runbook-<branch> -b <branch>`.
- **Branch names**: `m<N>/<topic>` for milestone work (`m1/discovery-walk`),
  otherwise `feat/`, `fix/`, `docs/`, `ci/`, `chore/`.
- **Exclusive file ownership.** Before editing, check open branches and PRs; when another
  agent's active branch touches a file, pick different work. Open a draft PR early and
  list the paths you will touch so other agents can see your claim.
- **Commits**: Conventional Commits (`feat(core): parse pnpm-workspace.yaml`).
- **PR description** includes: what and why, milestone, platforms where tests ran, and a
  justification for every new dependency.
- **Stack changes need an ADR first.** Write a short ADR in `docs/decisions/`
  (see its [README](docs/decisions/README.md)) and get it merged before changing the stack.

## Definition of done

A milestone (or a milestone on one platform) is done only when all of these hold:

1. Every CI job is green: `cargo fmt --all --check`,
   `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
   `pnpm check`, `pnpm build`.
2. Process and PTY behavior was tested on each platform the milestone claims, in CI or
   with a manual run recorded in the PR.
3. The acceptance criteria in `docs/ROADMAP.md` are met and its status table is updated.

Finish each milestone before starting the next.

## Code rules

- **Small modules**: one responsibility per file, aim for under ~300 lines per file and
  ~50 lines per function. Split before growing past that.
- **Comments explain why** and state constraints the code cannot show.
- **Rust**: library crates return typed errors (`thiserror` enums); `src-tauri` maps them
  to serializable IPC errors. Production code propagates errors with `?`; reserve
  `unwrap`/`expect` for tests and for invariants documented in a comment.
- **IPC**: Rust structs crossing IPC use `#[serde(rename_all = "camelCase")]`; their
  TypeScript mirrors live in `src/lib/ipc/`. Components call the typed wrappers.
- **TypeScript**: `strict` mode. Switches over unions or enums end in an exhaustive
  `never` check in `default`.
- **Imports** go at the top of each module.
- **Running scripts**: pass the script name and arguments as separate argv entries to the
  package manager (`pnpm run <name>`); build commands from argv, never from interpolated
  shell strings.

## Process and PTY code

Read [`docs/process-control.md`](docs/process-control.md) before touching anything that
spawns, signals or kills processes, or that reads or writes a PTY. Every OS-specific block
carries a comment explaining what it does on Linux, macOS and Windows.

## Dependencies

Add a dependency only with a written justification in the PR (why, alternatives
considered, size and maintenance status). The approved baseline is in
[ADR 0001](docs/decisions/0001-tech-stack.md); anything outside it needs that
justification.

## Resource usage

Runbook competes with IDEs on RAM, so resource use is a feature:

- **Event-driven at runtime**: blocking reads on dedicated threads, channels, OS
  notifications. A timer is acceptable only where the problem is inherently timed (kill
  grace period, output batching) and is documented at the call site.
- Stream PTY output through a Tauri `Channel`, batched, one channel per session.
- Bound everything that grows: xterm scrollback, output buffers, recent projects list.
- Dispose terminals and their listeners when a tab closes.

## Testing

- Unit tests live next to the code (`#[cfg(test)] mod tests`); integration tests in each
  crate's `tests/` directory, using projects under `fixtures/`.
- Tests are hermetic: no network, no dependence on the developer's machine; use
  `fixtures/` or temporary directories.
- Fix bugs test-first: a failing test that reproduces the bug, then the fix.
- Platform-specific tests are gated with `#[cfg(unix)]`, `#[cfg(target_os = "linux")]`,
  `#[cfg(windows)]`, etc., and run on the matching CI runner.
- The user's real pnpm + turbo monorepo is the acceptance test from M2 on. Tests that use
  it are `#[ignore]`, read its path from `RUNBOOK_ACCEPTANCE_REPO`, and run with
  `cargo test -- --ignored`.

## Known pitfalls

- **Fixtures contain `node_modules/` and `dist/` folders on purpose** to test that
  discovery skips them. The root `.gitignore` re-includes them under `fixtures/`; keep it
  that way when editing ignore rules. Git cannot store a nested `.git` directory, so tests
  that need one create it in a temporary directory at runtime.
- **GUI apps do not inherit the shell `PATH`** on Linux and macOS (nvm, fnm, volta,
  corepack live there). Resolve the user's login-shell environment before spawning
  scripts (M3).
- **Blank or black window on Linux**: first rule out fonts. WebKitGTK 2.52 spins its web
  process at 100% CPU, never navigates (inspector shows `about:blank`) and paints nothing
  when the OpenDyslexic WOFF fonts in `/usr/share/fonts/woff` (package
  `fonts-opendyslexic`, preinstalled on Zorin OS) are visible to fontconfig. It affects
  every WebKitGTK app. Runbook works around it at startup (`src-tauri/src/fonts.rs`,
  ADR 0006); fontconfig only applies `<glob>` patterns that start with `/`. Diagnosis is in
  the README troubleshooting section. GPU-related glitches (mostly NVIDIA) are a separate cause,
  worked around with `WEBKIT_DISABLE_DMABUF_RENDERER=1`; document any workaround where it
  is applied.
- **xterm.js** is published as `@xterm/xterm` and `@xterm/addon-*`; the unscoped `xterm`
  package is deprecated.
- **TypeScript stays on 6.x** until `svelte-check` supports TypeScript 7 (its peer range is
  `^5 || ^6`).
- **YAML goes through `yaml-rust2`** ([ADR 0004](docs/decisions/0004-yaml-parser.md));
  `serde_yaml` is deprecated.
