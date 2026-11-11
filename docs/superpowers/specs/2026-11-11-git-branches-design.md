# Basic Git branch control

Status: implemented; accepted on Linux. Proposed milestone: M9 (see
[`ROADMAP.md`](../../ROADMAP.md)), shipped in v0.4.0.

## Intent

The user switches branches constantly, every time they move to another task. In the IDE
they did it from the status bar without thinking; in Runbook they have to leave the app
(a terminal or the IDE) for it. Goal: see the current branch of each open project and
switch, fetch and pull from Runbook itself, the way an IDE status bar does.

Runbook stays light and fast: it drives the `git` command the user already has installed
and configured (credentials, SSH keys, credential helpers, hooks). Runbook never stores
credentials, never talks to GitHub directly and does not link a Git library.

## Goals

- Each open project shows its current branch.
- Switch to any local branch, or to a remote branch that has no local copy yet (a local
  tracking branch is created).
- Fetch: bring new remote branches and remote updates.
- Pull the current branch, fast-forward only.
- Uncommitted changes travel with the switch, after a warning. When Git refuses because
  they would be overwritten, Runbook shows Git's message and does nothing else.
- After a switch or a pull, the script list reflects the new `package.json` files, without
  stopping or touching running scripts.

## Non-goals

- Creating, renaming or deleting branches.
- Commits, staging, diffs, stash, merge, rebase, conflict resolution, push.
- Pull requests, issues or anything GitHub-specific (`gh`).
- Managing credentials or remotes. If Git needs credentials it cannot get on its own, the
  user fixes their Git setup outside Runbook.
- Watching the repository for changes made outside Runbook (file watchers, polling).

## Anti-goals (failure even if it "works")

- A Git operation that hangs forever waiting for a password or passphrase nobody can type.
- Losing or silently stashing uncommitted work.
- Switching branches stopping, restarting or clearing a running script.
- Runbook's background status refreshes making the user's own `git` commands fail with
  `index.lock` errors.
- A visible cost when Git is not used: projects without Git (or machines without `git`)
  show nothing and pay at most one failed `git` call when the project is shown.

## Behavior

### Where it shows

```
┌──────────────────────────────────────────────────────────────────────┐
│ …project view (sidebar, tabs, terminal)…                             │
├──────────────────────────────────────────────────────────────────────┤
│ ⎇ feat/login ↓2 ↑1 │ ● pnpm │ 6 packages, 24 scripts │ ~/code/montana│  ← status bar
└──────────────────────────────────────────────────────────────────────┘
```

- The branch is the first item of the project's status bar (`StatusBar.svelte`), the
  bottom-left corner where VS Code and JetBrains put it.
- It shows the branch name; in detached HEAD, the short commit hash instead.
- `↓N` (commits behind upstream) and `↑N` (commits ahead) appear only when non-zero and
  only when the branch has an upstream. They reflect the last fetch.
- The item is rendered only when the project root **is** the top level of a Git work tree
  (`git rev-parse --show-toplevel` equals the root). A project in a subfolder of a larger
  repository, a folder inside a versioned home directory, a folder without Git, or a
  machine without `git` shows nothing. This keeps the feature away from repositories the
  user did not open as a project, and from switches that would delete the project folder
  itself. (Designer's call, for simplicity; widening it later is possible.)

### The branch menu

Clicking the branch opens a menu anchored above it:

```
┌──────────────────────────────────────┐
│ [Filter branches…]   [Fetch] [Pull]  │
│ ✓ feat/login                         │
│   main                               │
│   fix/tab-scrollbar                  │
│ ── Remote ────────────────────────── │
│   release/v0.2.0          origin     │
│   m8/project-terminal     origin     │
└──────────────────────────────────────┘
```

- A filter input, focused on open, filters by substring (case-insensitive).
- Local branches first, the current one marked and listed first. Then remote branches
  that have no local branch of the same name, labelled with their remote. `<remote>/HEAD`
  is never listed.
- Arrow keys move the selection, Enter switches to it, Escape or a click outside closes
  the menu.
- While an operation runs, its button shows a busy state and the menu ignores further
  actions for that project; the other open projects are unaffected.
- Errors (Git's own message, trimmed) are shown inside the menu until the next action or
  until it closes. Successful actions close the menu (switch) or update it in place
  (fetch, pull).
- Pull is disabled when the branch has no upstream or is in detached HEAD.

### Switching

1. On selecting a branch, Runbook reads the status again (one `git status` call).
2. If there are uncommitted changes (tracked or untracked), the menu shows an inline
   confirmation: "3 changed files will move to `main`. Switch / Cancel". No changes: no
   confirmation.
3. Runbook runs the switch. Git carries the changes when it can and refuses when they
   would be overwritten; on refusal the menu shows Git's message and nothing else happens.
4. On success: the status bar updates, the project is re-discovered so the script list
   matches the new branch, and the menu closes.

Running scripts keep running. Dev servers with file watching pick up the new files on
their own, which is the expected behavior.

### Fetch and pull

- Fetch updates every remote and prunes deleted remote branches, then refreshes the
  status (ahead/behind) and the branch list.
- Pull is fast-forward only. If the branch has diverged from its upstream, Git's error is
  shown; Runbook never creates merge commits or starts a rebase. On success, the project
  is re-discovered like after a switch.

### When the status refreshes

Without watchers or timers:

- when the project view becomes visible (selected, or the app starts with it open);
- when the window regains focus, for the visible project only;
- when the branch menu opens;
- after every Git action from Runbook.

Each status carries the commit `HEAD` points to. When a refresh finds a different branch
or commit than the last one seen for that project, the project is re-discovered too, so
the script list never shows another branch's scripts for long. A branch changed in an
external terminal or the IDE shows up when the user comes back to the window. A branch
changed with `git switch` in one of Runbook's own shell tabs (the window keeps focus) shows
up at the next trigger: switching projects, opening the menu, or refocusing the window.
That delay is a known limitation, accepted to avoid watching the repository.

## How it is built (decided)

### ADR

A short ADR (`docs/decisions/0008-git-through-the-cli.md`) records: Git operations run the
user's `git` executable instead of linking `git2`/libgit2 or `gix`, because it reuses the
user's credentials, SSH agent, helpers and config with zero extra dependencies or binary
size; the cost is a runtime requirement on `git` (optional: the feature hides itself) and
parsing its output, which is limited to porcelain formats. It merges with or before the
code, per the ADR process.

### New crate `crates/runbook-git`

- Pure Rust, no Tauri, no dependency on `runbook-core` or `runbook-process`. It receives
  the working directory and the environment to run `git` with (the login-shell
  environment, which `src-tauri` already has, so a Homebrew or custom `git` on `PATH` is
  found).
- Typed errors with `thiserror`, for example: `git` not found, not a repository, Git
  failed (with its trimmed stderr), timed out, unparseable output.
- Commands are built from argv, never shell strings. Branch names are passed after the
  options; a name starting with `-` is rejected before running Git.
- Operations, each a single `git` invocation:
  - **status**: `git status --porcelain=v2 --branch` gives branch name or detached commit,
    upstream, ahead/behind and the changed-file count in one call. Run with
    `GIT_OPTIONAL_LOCKS=0` so it never takes the index lock the user's own commands need.
  - **branches**: `git for-each-ref` over `refs/heads` and `refs/remotes` with a
    machine-readable `--format`; the crate removes `*/HEAD` and remote branches that
    already exist locally.
  - **switch**: `git switch <branch>` for a local branch; `git switch --track
    <remote>/<branch>` for a remote-only one (explicit, so it works even when several
    remotes have the same branch name).
  - **fetch**: `git fetch --all --prune`.
  - **pull**: `git pull --ff-only`.
- Never interactive: stdin is null and `GIT_TERMINAL_PROMPT=0` is set, so HTTPS
  credential prompts fail instead of waiting. On Linux and macOS the child is started in
  a new session (no controlling terminal), so `ssh` cannot prompt for a passphrase on the
  terminal Runbook was launched from. On Windows, Git Credential Manager may show its own
  window, which is the user's credential setup working as intended. Every OS-specific
  block carries the comment AGENTS.md requires.
- Network operations (fetch, pull) have a timeout as a safety net against a hung remote or
  helper; the child is killed when it expires. This timer is inherently timed and is
  documented at the call site. Local operations (status, branches, switch) have none.
- Git's messages are shown as Git prints them, in the user's locale; only porcelain output
  is parsed, and that is locale-independent.

### Backend (`src-tauri`)

- Commands: `git_status(root)`, `git_branches(root)`, `git_switch(root, branch)`,
  `git_fetch(root)`, `git_pull(root)`. `root` must be an open project (existing
  `ProjectNotOpen` error otherwise). They run Git off the async runtime thread.
- `git_switch` and `git_pull` re-run discovery for the project after success and return the
  refreshed `Project` together with the new status, using the existing
  `OpenProjects::open` refresh path, which keeps running sessions attached.
- `git_status` on a root that is not a work tree's top level, or without `git`, returns a
  "no Git" value rather than an error, so the frontend simply hides the item. The other
  commands refuse such a root with a typed error.
- When a status refresh detects a new branch or commit (see "When the status refreshes"),
  the frontend asks for re-discovery through the existing `open_project` path; no separate
  command is needed. Whether `git_status` itself returns the refreshed `Project` in that
  case is the builder's call.
- If a switch or pull succeeds but re-discovery fails (for example a broken
  `package.json` on the new branch), the command still reports success with the new
  status, and the discovery error is shown the way opening errors are shown today. The
  Git operation is never reported as failed when it did happen.
- At most one Git operation per project runs at a time on the backend; a second request
  for the same project while one runs is rejected with a typed "busy" error.
- The crate's errors map to `CommandError` variants; IPC structs use
  `#[serde(rename_all = "camelCase")]` and their TypeScript mirrors and wrappers live in
  `src/lib/ipc/` (a new `git.ts`).

### Frontend

- `StatusBar` gets a small branch component (new file) that owns the status, the menu and
  the busy/error state for the visible project. The menu is its own component.
- `App.svelte` learns to replace an open project's data in place (same root, new
  `Project`), without remounting its `ProjectView`, so tabs and terminals are untouched.
  The branch component reports refreshed projects up through a callback.
- Refresh triggers as listed above: a window `focus` listener and the visible-project
  change, nothing periodic.
- Exhaustive `never` checks on any new union switches.

### Constraints from the repository rules

- One branch (`m9/git-branches`), Conventional Commits, PR listing platforms tested.
- Files under ~300 lines, functions under ~50.
- No new dependencies are expected in the crate beyond `thiserror` (already in the
  workspace). Anything else needs the written justification.

## Testing and acceptance

- Crate unit tests for the parsers (porcelain v2 status in its variants: normal branch,
  detached HEAD, no upstream, ahead/behind, changed and untracked files; `for-each-ref`
  output with `HEAD` and duplicated remote branches).
- Crate integration tests, hermetic, on all three OSes: create a bare repository as the
  remote and clones of it inside a temporary directory (`git init`, local file remote, no
  network), with `user.name`/`user.email` set in the test repos so the runner's config
  does not matter. Cover: status, a subfolder of a repository reported as "no Git", listing, switching local and remote-only branches,
  switching with changes that travel, switching refused by a conflicting change (work tree
  left intact), fetch picking up a new remote branch and pruning a deleted one, pull
  fast-forward, pull refused on a diverged branch, a folder that is not a repository.
- A test that a credential prompt fails instead of hanging (for example a remote URL that
  requires HTTPS auth served by nothing, or `GIT_ASKPASS` pointing at a failing helper),
  as long as it stays hermetic.
- `pnpm check`, `pnpm build` and every CI job green.
- Manual acceptance on Linux, recorded in the PR: in a real repo with a running `dev`
  script, switch branches from the menu with and without uncommitted changes, trigger a
  refused switch, fetch a branch created on GitHub, pull new commits, and confirm the
  script kept running and the script list changed when the branch had different scripts;
  open a non-Git folder and confirm nothing shows.
- `docs/ROADMAP.md`: a new M9 "Git branches" section with these criteria and a status
  row; the v1 out-of-scope line about Git is reworded to say basic branch control arrived
  in M9 and commits, merges and pull requests remain out of scope.

## Left to the builder

- Exact timeout length for fetch and pull (in the range of one to a few minutes).
- Data shapes of the status and branch DTOs.
- How the busy guard per project is stored.
- Menu styling within the current look, and whether a branch icon is a glyph or CSS.
- Whether the remote section is collapsible.
