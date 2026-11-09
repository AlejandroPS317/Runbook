# 0006 – Work around the WebKitGTK WOFF font hang on Linux

- Status: Accepted
- Date: 2026-11-09

## Context

With WebKitGTK 2.52, the web process spins at 100% CPU and never paints when fontconfig
exposes the OpenDyslexic WOFF fonts from `fonts-opendyslexic` (`/usr/share/fonts/woff`),
which Zorin OS installs by default. Every WebKitGTK app shows a blank window; diagnosis is
in the README troubleshooting section. Asking each user to write a fontconfig file is a
poor first-run experience.

## Decision

On Linux, at startup and before any thread or GTK is created, Runbook writes a small
fontconfig file to `$XDG_CACHE_HOME/runbook/fontconfig.conf` (default
`~/.cache/runbook/`) that includes the system configuration (`/etc/fonts/fonts.conf`,
which also loads the user's own configuration) and rejects WOFF and WOFF2 files (globs
`/*.woff` and `/*.woff2`; fontconfig ignores globs that do not start with `/`),
then points `FONTCONFIG_FILE` at it for the Runbook process and its WebKit helpers.

- Skipped when `FONTCONFIG_FILE` is already set (the user or a packager decided) or when
  `/etc/fonts/fonts.conf` does not exist.
- Scripts started from Runbook do not inherit the override: `FONTCONFIG_FILE` is removed
  from their environment when it points at Runbook's file.

## Alternatives considered

- **Documentation only**: keeps the blank window for every Zorin user on first launch.
- **Reject only `/usr/share/fonts/woff`**: matches the observed trigger exactly, but WOFF
  fonts installed elsewhere may trigger the same hang; desktop UI does not need web-font
  formats.
- **Disable fontconfig caches or force a minimal font set**: changes the UI's fonts for
  every user.

## Consequences

- Runbook renders on affected systems without user action.
- Runbook's own UI cannot use WOFF/WOFF2 fonts installed on the system (it does not need
  them).
- Remove the workaround once WebKitGTK fixes the hang; report it upstream.
