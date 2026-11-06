# 0003 – Dual MIT / Apache-2.0 license

- Status: Accepted
- Date: 2026-11-06

## Context

Runbook must be fully open source. Its main dependencies (Rust standard ecosystem,
Tauri, `portable-pty`, xterm.js, Svelte) are licensed under MIT and/or Apache-2.0.

## Decision

Runbook is licensed under `MIT OR Apache-2.0`, at the user's option, with the texts in
`LICENSE-MIT` and `LICENSE-APACHE`. Crates declare `license = "MIT OR Apache-2.0"`.

## Alternatives considered

- **MIT only**: simplest, but no explicit patent grant.
- **Apache-2.0 only**: patent grant, but incompatible with GPLv2-only projects.
- **GPL-3.0**: guarantees derivatives stay open, but blocks reuse of Runbook's crates in
  permissively licensed projects and departs from the ecosystem's convention.

## Consequences

- Same license model as Rust and Tauri, so users and contributors already understand it.
- The Apache-2.0 option provides an explicit patent grant.
- When contributions open, contributions are dual-licensed under the same terms by default
  (Apache-2.0 section 5).
