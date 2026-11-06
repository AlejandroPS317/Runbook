# 0002 – Linux as the primary target

- Status: Accepted
- Date: 2026-11-06

## Context

Runbook targets Linux, macOS and Windows, but the user develops and uses it on Linux
(Zorin OS 18, Ubuntu 24.04 base). The riskiest code (PTY handling and process-tree
termination) is different on each OS, and building all three at once would slow down
every milestone.

## Decision

Linux is the primary platform.

- Every milestone is implemented and accepted on Linux first.
- CI runs on Linux, macOS and Windows from M0. Platform-independent code (discovery,
  parsing, workspaces, UI build) must pass on all three at all times.
- Platform-specific code (PTY, process termination, installers) is written for Linux
  first. macOS shares the Unix path and follows next; Windows (ConPTY, Job Objects)
  follows after.
- `docs/ROADMAP.md` tracks each milestone per platform. A milestone is done on a platform
  only after its tests passed there.
- Until a platform-specific feature exists for macOS or Windows, it fails with a clear
  "not supported yet on this platform" error, so builds stay green and nothing fails
  silently.

## Alternatives considered

- **All three platforms in lockstep**: closest to the original plan, but Windows process
  control would block Linux progress on every milestone from M3 on.
- **Linux only, other platforms later**: faster at first, but portability bugs in shared
  code would pile up unseen; keeping all three in CI prevents that cheaply.

## Consequences

- The Linux experience reaches usable state sooner.
- macOS and Windows parity becomes explicit follow-up work per milestone.
- Linux packaging (AppImage, deb) comes first in M6.
