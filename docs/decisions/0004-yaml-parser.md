# 0004 – YAML parser for pnpm-workspace.yaml

- Status: Accepted
- Date: 2026-11-06

## Context

M2 reads the `packages` list from `pnpm-workspace.yaml`. Real files also contain
`catalog`/`catalogs` maps with quoted keys, comments and nested mappings, so a hand-written
line parser is not reliable. `serde_yaml`, the usual choice, is archived and deprecated.

## Decision

Use `yaml-rust2` 0.13 with default features disabled (the `encoding` feature pulls
`encoding_rs`; Runbook already reads files as UTF-8). Runbook only needs a document tree
and one key, so no `serde` integration is required.

## Alternatives considered

- **`serde_yaml`**: deprecated by its author since 2024.
- **`serde_yml`**: pre-1.0 fork with contested maintenance history.
- **`serde_norway`, `serde_yaml_ng`**: maintained forks of `serde_yaml`, but no release since
  2024 and they bring `unsafe-libyaml`; serde mapping is unnecessary here.
- **`saphyr`**: successor of `yaml-rust2` by the same maintainers, still pre-1.0 with a
  smaller user base.
- **Hand-written parser**: small, but breaks on flow sequences, quoting and comments.

## Consequences

- Pure Rust, small dependency set (`arraydeque`, `hashlink`), actively maintained
  (release in 2026, ~22M recent downloads).
- If `saphyr` reaches 1.0 and `yaml-rust2` is retired, migration is local to
  `crates/runbook-core/src/workspace.rs`.
