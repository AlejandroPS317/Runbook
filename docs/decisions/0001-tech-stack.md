# 0001 – Technology stack

- Status: Accepted
- Date: 2026-11-06

## Context

Runbook must be a light, fast, cross-platform desktop app that runs `package.json` scripts
in real terminals. The main competitor for the user's RAM is an IDE, so an Electron app
(a bundled Chromium plus Node per app) defeats the purpose. Implementation is done by AI
agents, so the stack should be well documented and widely known.

## Decision

| Concern                    | Choice                                                     |
| -------------------------- | ---------------------------------------------------------- |
| Core logic                 | Rust (stable), Cargo workspace                             |
| Desktop shell              | Tauri 2 (system webview: WebKitGTK, WKWebView, WebView2)   |
| Frontend                   | Svelte 5 + Vite + TypeScript, without SvelteKit            |
| Frontend package manager   | pnpm                                                       |
| Terminal UI                | `@xterm/xterm` with `@xterm/addon-fit`                     |
| PTY                        | `portable-pty` (Unix PTY; ConPTY on Windows)               |
| JSON                       | `serde`, `serde_json`                                      |
| Directory walk and globs   | `ignore` (parallel walker) and `globset`                   |
| Errors                     | `thiserror` in library crates                              |
| Test helpers               | `tempfile` (dev-dependency)                                |
| CI                         | GitHub Actions, matrix ubuntu/macos/windows-latest         |

This table is the approved dependency baseline referenced by AGENTS.md. Platform crates
for process control (`nix`/`libc` on Unix, `windows-sys` on Windows) are expected in M4
and are justified in that milestone's PR or ADR.

Supported target package managers in v1: pnpm and npm.

## Alternatives considered

- **Electron**: mature terminal ecosystem, but high idle RAM and large installers.
- **React** instead of Svelte: larger ecosystem, but a bigger runtime and more re-render
  management; Runbook's UI is small and Svelte's compiled output is lighter.
- **SvelteKit**: adds routing and SSR machinery that a single-window desktop app does not
  need; plain Vite keeps the build simpler.
- **`walkdir`**: simpler, but single-threaded; `ignore` walks in parallel, which matters for
  "hundreds of packages must feel instant".

## Consequences

- Rendering differs per OS webview; the UI must avoid engine-specific features and be
  checked on WebKitGTK first (Linux priority).
- Linux builds need WebKitGTK 4.1 development packages (documented in the README and CI).
- The YAML crate for `pnpm-workspace.yaml` is still open: `serde_yaml` is unmaintained, so
  M2 picks one in its own ADR.
