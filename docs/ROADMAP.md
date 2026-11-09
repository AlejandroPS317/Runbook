# Roadmap

Each milestone ends with passing CI before the next one starts. Linux is accepted first
([ADR 0002](decisions/0002-linux-first.md)); macOS and Windows are tracked separately.

## Status

Legend: ✅ done · 🚧 in progress · ⏳ not started · — not applicable

| Milestone              | Linux | macOS | Windows |
| ---------------------- | ----- | ----- | ------- |
| M0 Skeleton            | ✅    | 🚧    | 🚧      |
| M1 Discovery           | ✅    | ✅    | ✅      |
| M2 Monorepo workspaces | ✅    | ✅    | ✅      |
| M3 One terminal        | ✅    | 🚧    | 🚧      |
| M4 Stop and restart    | ✅    | ✅    | ✅      |
| M5 Multiple tabs       | ✅    | ✅    | ✅      |
| M6 Polish and packages | ✅    | 🚧    | 🚧      |
| M7 Multiple projects   | 🚧    | 🚧    | 🚧      |
| M8 Project terminal    | 🚧    | ⏳    | ⏳      |

M0 and M3 on macOS and Windows: builds, lints and tests (including PTY session tests) pass
in CI; the app has not been run on a real machine yet.

M6: installers build on all three OSes (Release workflow); the Linux `.deb` was installed
and run by the user. Pending on macOS and Windows: installing on a real machine.

In-app updates ([ADR 0007](decisions/0007-in-app-updates.md)): on Linux an AppImage
updated itself from 0.1.1 to 0.1.2 against a local manifest, including stopping a running
script first. Pending: a `.deb` update from a published release (Linux), and a first
update on macOS and Windows.

## v1 scope

In scope: open a folder, recent projects, discover `package.json` files, respect pnpm and
npm workspaces, list scripts grouped by package with a search filter, run each script in
a real PTY terminal tab, stop and restart with full process-tree termination, clean
shutdown, keyboard shortcuts, installers for Linux, macOS and Windows.

Out of scope for v1: editing files, installing or updating dependencies, Git or pull
requests, yarn, bun and Lerna, cloud sync, accounts, telemetry.

## Milestones

### M0 – Skeleton

- Cargo workspace with `src-tauri` and an empty `crates/runbook-core`.
- Tauri 2 app with a Svelte 5 + Vite + TypeScript frontend that opens an empty window.
- GitHub Actions CI on `ubuntu-latest`, `macos-latest`, `windows-latest` running the
  gate from AGENTS.md (fmt, clippy, tests, `pnpm check`, `pnpm build`).
- `README.md`, `AGENTS.md`, `CLAUDE.md`, licenses.

Accepted when: `pnpm tauri dev` opens a window on Linux and CI is green on all three OSes.

### M1 – Discovery

- `runbook-core` exposes a function that, given a folder, returns every `package.json`
  found and its scripts (name → command), with paths relative to the root.
- Skips `node_modules`, `.git`, `dist`, `build`, `out`, `coverage`, `.next`, `.turbo`,
  `.svelte-kit`, `.cache`, `target` and similar.
- Tolerates invalid JSON and `package.json` files without `scripts`: reported, not fatal.
- `fixtures/` contains a sample pnpm + turbo monorepo (see below).

Accepted when: unit and integration tests over `fixtures/` pass on all three OSes.

### M2 – Monorepo workspaces

- Reads `pnpm-workspace.yaml` and the `workspaces` field of the root `package.json`
  (array and object forms), including negated globs.
- Groups scripts by package (name and `package.json` path); the root package is its own
  group.
- Detects the package manager: `packageManager` field first, then `pnpm-workspace.yaml` or
  `pnpm-lock.yaml` → pnpm, otherwise npm.
- ADR choosing the YAML crate.

Accepted when: fixture tests pass on all three OSes and the user's real monorepo
(`RUNBOOK_ACCEPTANCE_REPO`) is grouped correctly on Linux.

### M3 – One terminal

- `crates/runbook-process` spawns a script in a PTY and streams output to xterm.js.
- Login-shell environment resolution so `pnpm`/`node` from nvm, fnm, volta or corepack are
  found when the app is launched from the desktop.
- Terminal resize propagates to the PTY.

Accepted when: running `dev` from the fixture and from the real monorepo shows live,
colored output on Linux.

### M4 – Stop and restart

- Stop terminates the whole process tree as specified in
  [`process-control.md`](process-control.md); restart reuses command and cwd.
- Platform-specific tests per that document.

Accepted per platform when its process-tree tests pass on that platform's CI runner and
`turbo run dev` in the real monorepo leaves no processes behind (Linux).

### M5 – Multiple tabs

- Several scripts run at once, one tab each; switching tabs keeps each terminal's state.
- Closing a tab stops its process; quitting the app stops all of them.

Accepted per platform when a test spawns several sessions, quits the app's session
manager, and finds no surviving processes.

### M6 – Polish and packages

- Recent projects (bounded list, stored locally), script search filter, keyboard
  shortcuts.
- Installers: AppImage and deb (Linux), dmg (macOS), msi/exe (Windows).
- Measure idle RAM with a project open and record it in the README (done: ~241 MiB PSS on
  Linux).
- Mitigate the WebKitGTK hang caused by WOFF fonts on Linux in the app itself (done:
  [ADR 0006](decisions/0006-linux-woff-font-workaround.md)).
- In-app updates from signed GitHub releases, with a startup check the user can turn off
  ([ADR 0007](decisions/0007-in-app-updates.md)).

Accepted per platform when its installer installs and runs on a clean machine without
extra setup.

### M7 – Multiple projects

Design: [`superpowers/specs/2026-11-09-multiple-projects-design.md`](superpowers/specs/2026-11-09-multiple-projects-design.md).

- Several projects open in one window, selected from a project bar (Alt+1…9, ⌘1…9 on
  macOS); each keeps its script list, filter and terminal tabs.
- Switching projects never stops or clears a script; each project shows how many of its
  scripts are running and how many failed.
- Closing a project stops exactly its scripts (`SessionManager::stop_many`); quitting stops
  all of them. Startup still shows the home screen (restoring projects is out of scope).

Accepted per platform when the session manager tests (`stop_many` leaves the other
sessions running) pass on that platform's CI runner, and on Linux three real projects run
scripts side by side, closing one leaves no process of it behind and keeps the others
running.

### M8 – Project terminal

- Open an interactive shell tab in the project root or in any package folder, for commands
  that are not scripts. The shell is the user's (`$SHELL`), with the login-shell
  environment.
- Open a script "in a terminal": a shell in the package folder with `<pm> run <script> `
  already typed at the prompt, so the user can add arguments (for example `--apply`)
  before pressing Enter. Runbook never runs that line itself.
- Shell tabs stop, restart and close like script tabs, leaving no process behind.

Accepted per platform when the interactive-shell session tests pass on that platform's CI
runner, and on Linux a real monorepo script runs both from ▶ and from a prefilled shell
with extra flags, and closing shell tabs leaves no processes behind.

## Reference fixture

`fixtures/` mirrors the shape of the user's real monorepo:

- Root `package.json` with scripts that call `turbo run dev` (and `build`, `test`).
- Packages under `apps/` and `packages/` with `dev`, `build` and `test` scripts.
- Scripts using pnpm filters, for example `pnpm --filter @scope/web dev`.
- Decoy `package.json` files inside `node_modules/` and `dist/` that discovery must skip.
