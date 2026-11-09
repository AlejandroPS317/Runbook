# Multiple open projects in one window

Status: design, awaiting approval. Proposed milestone: M7 (after v1 scope, see
[`ROADMAP.md`](../../ROADMAP.md)).

## Intent

The user works on several independent projects at the same time (for example
`pinprexat-monorepo`, `pinprexat-app` and `montana`, each its own folder and repo). Today
Runbook holds exactly one project: opening another folder (Ctrl+O or a recent project)
replaces the current one and stops all of its scripts. The only way to keep several
projects running is to open several Runbook windows, which is what the user wants to stop
doing.

Goal: keep several projects open in a single window, each with its own scripts grouped by
package, exactly as one project looks today, and switch between them without stopping
anything.

## Goals

- Several projects open at once in one window. Each keeps its own script list, filter
  text, terminal tabs and active tab.
- Switching projects never stops, restarts or clears anything. Scripts in hidden projects
  keep running and keep their output.
- The project bar shows, per project, how many scripts are running and how many failed, so
  a crash in a hidden project is visible at a glance.
- Only the selected project's content is shown below the project bar.
- Closing a project stops all of its scripts (whole process tree, as today) and nothing
  else. Quitting the app stops every script of every project, as today.

## Non-goals

- Restoring the open projects when Runbook starts again. Startup always shows the home
  screen; restoring sessions is a separate, later task.
- A visual redesign. The user plans to modernize the UI later; this change reuses the
  current look and components. Function over form.
- Multiple windows, split views, or showing two projects' terminals side by side.
- A limit on the number of open projects.

## Anti-goals (failure even if it "works")

- Any script stopping, restarting or losing its output because the user switched projects
  or opened another one.
- Closing project A stopping, or leaving orphaned, processes of project B (or of A).
- Keyboard shortcuts acting on a hidden project (for example Ctrl+Shift+W closing a tab of
  a project that is not on screen).
- Opening a project that is already open creating a second copy of it.

## Behavior

### Layout

```
┌───────────────────────────────────────────────────────────────────┐
│ pinprexat-monorepo ●2 │ pinprexat-app ●1 ●1 │ montana │  +        │  ← project bar
├──────────────────┬────────────────────────────────────────────────┤
│ montana  pnpm  ✕ │ montana › dev × │ web › build ×                │  ← montana's script tabs
│ [Filter scripts] │────────────────────────────────────────────────│
│ root             │                                                │
│   dev        ▶   │   terminal of montana's active script tab      │
│ apps/web         │                                                │
│   build      ▶   │                                                │
└──────────────────┴────────────────────────────────────────────────┘
```

- A **project bar** sits at the top of the window (below the update banner), with one
  entry per open project in opening order, and a `+` button at the end.
- Each entry shows the project name, a close button (×), and the status preview:
  - a green dot with the number of the project's script tabs whose status tone is
    `active` (starting, running or stopping), shown only when > 0;
  - a red dot with the number of tabs whose tone is `error` (failed to start, or exited
    with a non-zero code), shown only when > 0.
  Tones come from `statusTone` in `src/lib/terminal/status.ts`; scripts the user stopped
  are `idle` and show nothing.
- The entry's tooltip shows the project's full path, since two projects can share a
  folder name.
- Everything below the bar is the existing `ProjectView` for the selected project,
  unchanged in content.
- With no projects open there is no project bar: the window shows the home screen exactly
  as today.

### Opening projects

- From the home screen (first open, or `+`): picking a folder or a recent project adds it
  to the open projects and selects it.
- Ctrl+O (⌘O) from a project opens the folder picker directly, as today, but the chosen
  project is **added** and selected instead of replacing the current one.
- `+` shows the home screen (recent projects, "Open folder…", version and update footer) in
  place of the project content, with the project bar still visible and no project entry
  highlighted. Choosing a project there adds it; selecting an existing project entry
  leaves the home screen. Escape returns to the previously selected project if it is still
  open, otherwise to the first project in the bar.
- If the chosen folder is already open (same `root` as returned by the backend), Runbook
  selects the existing entry. It does not create a second entry, re-mount the view or
  touch its terminals. Builder's call whether the existing script list is refreshed with
  the new discovery; keeping it as-is is acceptable.
- Every successful open still records the project in the recent list.
- Opening errors from the home screen are shown in its error line, as today. For Ctrl+O
  from a project, the error is shown inside the selected project's view (a dismissible
  line, for example under the sidebar header) and the open projects are left untouched.
- The command-line folder (`runbook <folder>`) still opens once at startup.

### Switching projects

- Clicking an entry selects that project.
- Shortcuts: Alt+1 … Alt+9 on Linux and Windows, ⌘1 … ⌘9 on macOS select the 1st … 9th
  project in the bar. Match on `event.code` (`Digit1`…`Digit9`) so keyboard layouts do
  not matter. These are added to `src/lib/keyboard.ts` (which today returns `null` for
  any Alt combination; that early return must allow this case only). On Linux and Windows
  the shortcut requires Alt with no Ctrl, Shift or Meta: Windows reports AltGr as
  Ctrl+Alt, and AltGr+digit types characters such as `@` on Spanish layouts.
- Existing shortcuts keep their meaning and apply **only to the selected project**:
  Ctrl+Tab / Ctrl+Shift+Tab / Ctrl+PageUp/PageDown cycle that project's script tabs;
  Ctrl+Shift+W/R/K and Ctrl+Shift+F act on it. While the home screen is shown via `+`,
  project shortcuts do nothing except Ctrl+O and the project-switch shortcuts.
- When a project becomes visible again, its active terminal must re-fit to the current
  window size (the window may have been resized while it was hidden) and take focus,
  the same way a script tab does today when it is selected.

### Closing projects

- The project entry's × and the existing "Close" button in the sidebar header both close
  the project: all its scripts are stopped (process tree), its terminals disposed, and the
  entry removed. No confirmation dialog, consistent with closing a script tab today.
- After closing the selected project, select the neighbor entry (the next one, or the
  previous one if it was last). After closing the last project, show the home screen.
- Closing a hidden project's entry does not change the selected project.

### Quitting and updating

Unchanged: quitting stops every session of every project (`SessionManager::stop_all` on
`RunEvent::Exit`), and the in-app update's "running scripts" count already covers all
sessions, so it automatically counts every project.

## How it is built (decided)

### Backend (`src-tauri`)

- `AppState.project: Mutex<Option<Discovery>>` becomes a collection of open projects keyed
  by their root (the `root` string returned in `ProjectDto`).
- `open_project(path)` discovers and inserts (or replaces the snapshot of) that project; it
  no longer drops the others.
- New command `close_project(root)`: removes the project and stops **all of its sessions
  on the backend** (in parallel, like `stop_all`, off the async runtime thread), then
  forgets them. Process cleanup must not depend on the frontend unmounting terminals; the
  frontend's own `closeTerminal` calls for those sessions are then harmless no-ops
  (`close_terminal` already ignores unknown ids).
- `run_script` takes the project root as an extra argument and resolves the package and
  script inside that project. A root that is not open is a new typed error (replacing
  `CommandError::NoProject`), surfaced in the terminal like other spawn errors.
- The backend records which project each session belongs to, so `close_project` can find
  them. A `run_script` that finishes spawning after its project was closed must not leave
  a running session behind: it stops the new session and returns the "project not open"
  error. `runbook-process` stays unaware of projects (no project concept in that crate);
  the association lives in the Tauri side or in a small, window-independent module that
  can be unit-tested.
- IPC rules apply: new or changed structs use `#[serde(rename_all = "camelCase")]`, and
  their TypeScript mirrors and typed wrappers live in `src/lib/ipc/`.

### Frontend

- `App.svelte` holds the list of open projects, the selected one, and whether the home
  screen is shown. It renders the project bar, then every open project's `ProjectView` at
  once, hiding all but the selected one (same technique `ProjectView` uses for terminal
  slots). Views must stay mounted while hidden: unmounting a view stops its scripts.
- `ProjectView` gets an `active` (visible) flag. Its window-level keyboard handler acts only
  when it is active, and a terminal counts as active only if both its tab and its project
  are active.
- `ProjectView` reports its tabs' status counts upward (callback or shared state, builder's
  call) for the project bar's preview.
- `ProjectView` stops replacing itself: the Ctrl+O path calls back into `App` to add a
  project. The `{#key project.root}` remount trick goes away.
- The home screen gets the same `onOpen` callback; whether it is rendered once and toggled
  or mounted on demand is the builder's call, as long as `takeInitialPath` is consumed
  only once and Ctrl+O is not handled twice (home screen and a hidden project view).
- New component for the project bar, kept small, reusing `TabBar`'s look where practical.
  It may share code with `TabBar` or not; builder's call.

### Constraints from the repository rules

- One branch, Conventional Commits, PR with platforms tested (AGENTS.md).
- Files under ~300 lines, functions under ~50; split rather than grow `App.svelte` or
  `ProjectView.svelte` past that.
- Exhaustive `never` checks on any new `Action` switch cases.
- No new dependencies are expected.

## Testing and acceptance

- Rust unit tests: opening two projects keeps both; `run_script` resolves packages per
  project and rejects a root that is not open; `close_project` removes only that project.
- Rust integration test (process behavior, gated per platform like the M5 tests): register
  sessions for two projects (spawned with simple shell commands, not the package manager,
  so the test stays hermetic), close one project, verify its process trees are gone and
  the other project's sessions are still running.
- `pnpm check` and `pnpm build` pass; all CI jobs green.
- Manual acceptance on Linux, recorded in the PR: open `pinprexat-monorepo`,
  `pinprexat-app` and `montana`; run `dev` in two of them; switch back and forth (click
  and Alt+number) and confirm output keeps streaming and the preview counts are right; make
  one script fail and see the red count on its hidden project; close one project and
  confirm with `ps` that only its processes are gone; quit and confirm nothing is left.
- `docs/ROADMAP.md` gets an M7 "Multiple projects" section with these acceptance criteria
  and a status row, updated when accepted.

## Left to the builder

- Exact data structures for the open-project registry and the session-to-project map.
- How status counts flow from `ProjectView` to the project bar.
- Whether the home screen is kept mounted or re-created when `+` is pressed.
- Whether re-opening an already open project refreshes its script list.
- Styling details of the project bar, within the current look.
