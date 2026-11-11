<h1 align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
    <img src="assets/logo.svg" alt="Runbook" width="360">
  </picture>
</h1>

A lightweight desktop app to run and manage `package.json` scripts in JavaScript projects
and monorepos, without opening an IDE.

Open a folder and Runbook finds every `package.json` (respecting pnpm and npm workspaces),
lists the scripts grouped by package, and runs each one in its own real terminal tab with
live output. Every tab can stop or restart its script, and stopping kills the whole
process tree, so tools like `turbo`, `pnpm` and `vite` leave no orphan processes.

Built with Rust, [Tauri 2](https://tauri.app), Svelte 5 and xterm.js, so it uses the
system webview instead of bundling a browser.

## Status

**v0.3.1 (pre-release).** The v1 feature set is complete and tested on Linux, the primary
platform. macOS and Windows builds pass the full test suite in CI but have not been run on
real machines yet. See [`docs/ROADMAP.md`](docs/ROADMAP.md) for per-platform status.

## Install

**Requirements:** Node.js with npm or pnpm, as you already use for your projects. Runbook
runs your scripts with them; nvm, fnm, volta and corepack setups are found automatically.

Download the installer for your system from
[Releases](https://github.com/AlejandroPS317/Runbook/releases) in the browser, or use the
commands below with the [GitHub CLI](https://cli.github.com) (run `gh auth login` once).
Replace `v0.3.1` with the version you want.

### macOS (Apple Silicon)

```sh
gh release download v0.3.1 -R AlejandroPS317/Runbook -p '*.dmg' -D ~/Downloads
hdiutil attach -nobrowse -mountpoint /tmp/runbook-dmg ~/Downloads/Runbook_*_aarch64.dmg
cp -R /tmp/runbook-dmg/Runbook.app /Applications/
hdiutil detach /tmp/runbook-dmg
xattr -dr com.apple.quarantine /Applications/Runbook.app
open /Applications/Runbook.app
```

The `xattr` line is needed because Runbook is not notarized by Apple yet: without it macOS
says the app "is damaged" or "cannot be verified". Intel Macs are not supported yet
(check with `uname -m`: `arm64` is Apple Silicon).

### Linux: Debian, Ubuntu, Zorin, Mint…

```sh
gh release download v0.3.1 -R AlejandroPS317/Runbook -p '*.deb'
sudo apt install ./Runbook_*_amd64.deb
```

Open **Runbook** from the applications menu, or run `runbook` (optionally with a folder:
`runbook ~/code/my-project`).

### Linux: any distribution (AppImage, no install)

```sh
gh release download v0.3.1 -R AlejandroPS317/Runbook -p '*.AppImage'
chmod +x Runbook_*_amd64.AppImage
./Runbook_*_amd64.AppImage
```

### Windows

In PowerShell:

```powershell
gh release download v0.3.1 -R AlejandroPS317/Runbook -p '*-setup.exe'
.\Runbook_0.3.1_x64-setup.exe
```

Windows SmartScreen warns because the installer is not signed yet: choose
**More info → Run anyway**. An `.msi` is also available for managed installs.

### Updates

Runbook updates itself from v0.1.2 on (v0.1.1 and older have to be reinstalled once with
the commands above). At startup it checks GitHub Releases for a newer version; when there
is one, a banner offers **Update and restart**. The update is downloaded, its signature
verified, and Runbook restarts on the new version. If scripts are running, Runbook asks
first and stops them (with their whole process tree) before installing. A `.deb` install
asks for your password, since it updates the system package. The home screen shows the
version and has **Check for updates** and a **Check at startup** switch to turn the
automatic check off.

### Uninstall

- macOS: delete `/Applications/Runbook.app`.
- Debian/Ubuntu: `sudo apt remove runbook`.
- AppImage: delete the file.
- Windows: Settings → Apps → Installed apps → Runbook → Uninstall.

Your recent projects list and settings live in the app data folder
(`~/.local/share/io.github.alejandrops317.runbook` on Linux,
`~/Library/Application Support/io.github.alejandrops317.runbook` on macOS,
`%APPDATA%\io.github.alejandrops317.runbook` on Windows).

### Building the installers

The [Release workflow](.github/workflows/release.yml) builds every installer for each
`v*` tag and attaches them to a draft GitHub release; a manual run builds them without
releasing, and pull requests build none. Locally, run
`pnpm tauri build --bundles deb,appimage` (Linux), `app,dmg` (macOS) or `msi,nsis`
(Windows). macOS builds are ad-hoc signed (`signingIdentity: "-"`), which Apple Silicon
needs for downloaded apps; real signing and notarization need an Apple Developer account.

To ship a new version
([ADR 0007](docs/decisions/0007-in-app-updates.md)):

1. Bump `version` in `package.json`, `src-tauri/tauri.conf.json` and the root
   `Cargo.toml` (for a minor or major bump, also the `runbook-core` and `runbook-process`
   requirements in `src-tauri/Cargo.toml`), merge, then tag `main`:
   `git tag v0.3.1 && git push origin v0.3.1`.
2. The workflow signs the update bundles with the updater key (`TAURI_SIGNING_PRIVATE_KEY`
   and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets) and attaches them with `latest.json`
   to the draft.
3. Review the draft and **Publish** it (not as a pre-release). Installed apps offer the
   update from then on.

Local builds skip the update bundles. To produce them, pass
`--config src-tauri/tauri.updater.conf.json` with the two signing variables set. Keep an
offline backup of the private key: without it, existing installs can never accept another
update.

### Memory use

Measured on Linux (Zorin OS 18, WebKitGTK 2.52) with the `fixtures/pnpm-turbo-monorepo`
project open and no script running, as PSS (shared pages split between processes) summed
over Runbook and its WebKit helpers:

| Build                    | Total   | runbook | WebKit web | WebKit network |
| ------------------------ | ------- | ------- | ---------- | -------------- |
| Release binary (`.deb`)  | 241 MiB | 100 MiB | 124 MiB    | 16 MiB         |
| AppImage                 | 255 MiB | 106 MiB | 125 MiB    | 22 MiB         |

Each running script adds its own processes (node, vite, …), which are not counted here.

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

Pick a folder with **Open folder…** (or a recent project), then click a script to run it
in its own terminal tab; the script list shows which scripts are running, passed or
failed. In monorepos with `apps/` and `packages/` folders, **Apps** and **Packages** narrow
the list. A built binary also accepts the folder on the command line:
`runbook /path/to/project`.

For commands that are not scripts, **+** in the tab strip (or `>_` next to a package)
opens your shell in the project or package folder. `>_` next to a script opens that shell
with `pnpm run <script> ` already typed, so you can add flags such as `--apply` before
pressing Enter.

Several projects can be open at once: **+** next to the project switcher (or Ctrl+O) adds
another one. Switching projects keeps every script running; the switcher's menu shows how
many of each project's scripts are running and how many failed, and closes projects.
Closing a project stops only its scripts.

Runbook follows the system's light or dark theme, terminals included.

### Keyboard shortcuts

On macOS, use ⌘ instead of Ctrl and instead of Alt (tab switching keeps Ctrl).

| Shortcut                      | Action                                         |
| ----------------------------- | ---------------------------------------------- |
| Ctrl+O                        | Open a folder as another project               |
| Alt+1 … Alt+9                 | Select the 1st … 9th open project              |
| Ctrl+Shift+F                  | Filter scripts (Enter runs the first match; Shift+Enter types it in a terminal) |
| Ctrl+Shift+T                  | Open a terminal in the project folder          |
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

### App icon and logo

The logo sources live in `assets/`: `icon.svg` (the hexagon), `logo.svg` and
`logo-dark.svg` (horizontal, for light and dark backgrounds). `src-tauri/icons/icon.svg` is
`assets/icon.svg` on a square canvas. Regenerate the platform icons with
`pnpm tauri icon src-tauri/icons/icon.svg`, then delete the generated `android/`, `ios/`,
`64x64.png` and `Square*`/`StoreLogo` files: Runbook only ships desktop builds.

## Troubleshooting

### Linux: the window opens but stays blank or black

WebKitGTK 2.52 (the system webview Tauri uses on Linux) hangs before painting anything
when the OpenDyslexic WOFF fonts from the `fonts-opendyslexic` package are installed;
Zorin OS ships them by default. Runbook works around it by itself: at startup it points
fontconfig at `~/.cache/runbook/fontconfig.conf`, which loads your normal configuration
and skips WOFF files ([ADR 0006](docs/decisions/0006-linux-woff-font-workaround.md)). Set
`FONTCONFIG_FILE` yourself to opt out.

Other WebKitGTK apps are still affected. To check, see whether
`fc-list | grep /usr/share/fonts/woff/` prints anything. Fix it for every app without
`sudo` by hiding only those files from fontconfig (the OTF version of OpenDyslexic stays
available):

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
