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

Pick a folder with **Open folder…** (or a recent project), then press ▶ next to a script
to run it in its own terminal tab. A built binary also accepts the folder on the command
line: `runbook /path/to/project`.

### Keyboard shortcuts

On macOS, use ⌘ instead of Ctrl (tab switching keeps Ctrl).

| Shortcut                      | Action                                         |
| ----------------------------- | ---------------------------------------------- |
| Ctrl+O                        | Open a folder                                  |
| Ctrl+Shift+F                  | Filter scripts (Enter runs the first match)    |
| Ctrl+Tab / Ctrl+Shift+Tab     | Next / previous tab (also Ctrl+PageDown/PageUp) |
| Ctrl+Shift+K                  | Stop the active tab's script and its processes |
| Ctrl+Shift+R                  | Restart the active tab's script                |
| Ctrl+Shift+W                  | Close the active tab (stops its script)        |

Shortcuts use Shift so the shell inside a tab keeps Ctrl+W, Ctrl+K and similar keys.

### Checks (same as CI)

```sh
pnpm check
pnpm build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Inspect discovery on a real project

```sh
cargo run -p runbook-core --example discover -- /path/to/project
```

Prints every package and script Runbook finds, any problems, and the time it took.

### App icon

The source icon is `src-tauri/icons/icon.svg`. Regenerate the platform icons with
`pnpm tauri icon src-tauri/icons/icon.svg`, then delete the generated `android/`, `ios/`
and `Square*`/`StoreLogo` files: Runbook only ships desktop builds.

## Troubleshooting

### Linux: the window opens but stays blank or black

WebKitGTK 2.52 (the system webview Tauri uses on Linux) hangs before painting anything
when the OpenDyslexic WOFF fonts from the `fonts-opendyslexic` package are installed;
Zorin OS ships them by default. Every WebKitGTK app is affected, not only Runbook. To
check, see whether `fc-list | grep /usr/share/fonts/woff/` prints anything.

Fix without `sudo` by hiding only those files from fontconfig (the OTF version of
OpenDyslexic stays available):

```sh
mkdir -p ~/.config/fontconfig/conf.d
cat > ~/.config/fontconfig/conf.d/50-reject-woff.conf <<'EOF'
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig>
  <selectfont>
    <rejectfont>
      <glob>/usr/share/fonts/woff/*</glob>
    </rejectfont>
  </selectfont>
</fontconfig>
EOF
```

Alternatively, remove the package: `sudo apt remove fonts-opendyslexic`.

If fonts are not the cause and you have an NVIDIA GPU, try
`WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm tauri dev`. See also
[Tauri's Linux graphics notes](https://v2.tauri.app/develop/debug/linux-graphics/).

## Working on Runbook

Runbook is built by AI agents (Claude Code and Cursor) under the rules in
[`AGENTS.md`](AGENTS.md). Architecture decisions are recorded in
[`docs/decisions/`](docs/decisions/). External contributions are not accepted yet; see
[`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option.
