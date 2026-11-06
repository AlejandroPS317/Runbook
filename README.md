# Runbook

A lightweight desktop app to run and manage `package.json` scripts in JavaScript projects
and monorepos, without opening an IDE.

Open a folder and Runbook finds every `package.json` (respecting pnpm and npm workspaces),
lists the scripts grouped by package, and runs each one in its own real terminal tab with
live output. Every tab can stop or restart its script, and stopping kills the whole
process tree, so tools like `turbo`, `pnpm` and `vite` leave no orphan processes.

Built with Rust, [Tauri 2](https://tauri.app), Svelte 5 and xterm.js, so it uses the
system webview instead of bundling a browser.

## Status

Early development; nothing is usable yet. Linux is the primary platform, with macOS and
Windows following. See [`docs/ROADMAP.md`](docs/ROADMAP.md) for milestones and
per-platform status.

## Development setup

### Prerequisites

- Rust (stable) via [rustup](https://rustup.rs)
- Node.js 20.19+ and pnpm 9+
- Platform dependencies for Tauri 2:

**Linux (Debian/Ubuntu and derivatives)**

```sh
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

For other distributions, see the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

**macOS**: Xcode Command Line Tools (`xcode-select --install`).

**Windows**: Microsoft C++ Build Tools and WebView2 (preinstalled on Windows 10 1803+ and
Windows 11).

### Run

```sh
pnpm install
pnpm tauri dev
```

### Checks (same as CI)

```sh
pnpm check
pnpm build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### App icon

The source icon is `src-tauri/icons/icon.svg`. Regenerate the platform icons with
`pnpm tauri icon src-tauri/icons/icon.svg`, then delete the generated `android/`, `ios/`
and `Square*`/`StoreLogo` files: Runbook only ships desktop builds.

## Working on Runbook

Runbook is built by AI agents (Claude Code and Cursor) under the rules in
[`AGENTS.md`](AGENTS.md). Architecture decisions are recorded in
[`docs/decisions/`](docs/decisions/). External contributions are not accepted yet; see
[`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option.
