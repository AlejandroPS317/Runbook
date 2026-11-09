# 0007 – In-app updates with the Tauri updater and GitHub Releases

- Status: Accepted
- Date: 2026-11-09

## Context

Runbook cannot update itself: users have to notice a release, download the installer for
their OS and reinstall it by hand, so most installs stay on old versions. Runbook ships on
Linux (`.deb`, AppImage), macOS (`.app` in a `.dmg`) and Windows (NSIS `.exe`, `.msi`), and
the repository and its releases are public on GitHub. Runbook competes with IDEs on
resource use and must not restart while scripts are running.

## Decision

Use the official Tauri updater, `tauri-plugin-updater` (2.x), with a static `latest.json`
attached to each GitHub release and fetched from
`https://github.com/AlejandroPS317/Runbook/releases/latest/download/latest.json`.

- **Signing**: the Release workflow signs every update bundle with a minisign key
  (`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets). The app
  embeds the public key and rejects bundles it did not sign. The private key has an
  offline backup; losing it means existing installs can never accept another update.
- **Bundles**: each install updates with its own format. The manifest has one entry per
  bundle type (`linux-x86_64-deb`, `linux-x86_64-appimage`, `darwin-aarch64-app`,
  `windows-x86_64-nsis`, `windows-x86_64-msi`); the plugin picks the one matching how the
  running app was installed. `.deb` installs are updated with `dpkg -i` behind a `pkexec`
  password prompt (verified in plugin 2.14), so no Linux install is left notify-only.
- **When**: one check at startup and a manual "Check for updates" action; no polling. A
  setting disables the startup check. Development builds never check on their own.
- **Install**: the update downloads while scripts keep running; Runbook then stops every
  session through the existing process-tree termination (`docs/process-control.md`),
  installs and restarts with `AppHandle::restart()`. The user confirms first and is told
  how many scripts will be stopped.
- **Publishing**: a `v*` tag still creates a draft release, now with the update bundles,
  their signatures and `latest.json`. Updates reach users when the maintainer publishes
  the draft (not as a pre-release), because `releases/latest` only resolves published
  releases.

## Alternatives considered

- **Notify and link to the download page**: no new dependency, but every update stays a
  manual reinstall, which is the problem.
- **System package channels (APT repository, Flathub, Homebrew cask, winget)**: native on
  each platform, but four separate channels to maintain, none of them cross-platform.
  They can complement the updater later.
- **`tauri-plugin-process` for the restart**: `AppHandle::restart()` already does it.
- **`tauri-apps/tauri-action` to build and write `latest.json`**: replaces the existing
  workflow; a short script over the signed artifacts does the same and keeps the build
  steps as they are.
- **Self-hosted update server**: dynamic responses are not needed; GitHub Releases is
  free, already used and public.

## Consequences

- Publishing a release is enough for installed apps to offer the update.
- New Rust dependencies: `tauri-plugin-updater` and its HTTP stack (`reqwest`, `rustls`,
  `minisign-verify`), a few MB of binary size. No new frontend dependency.
- The app contacts GitHub once at startup unless the user opts out; the README says so.
- Releases depend on the signing secrets; local `pnpm tauri build` stays unsigned because
  only the Release workflow enables updater artifacts.
- Release asset names must stay stable (`latest.json`, bundle file names); renaming them
  or moving releases breaks updates for every existing install.
