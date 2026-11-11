# Architecture Decision Records

An ADR records one technical decision: the context, the choice, and its consequences.
Write one before changing the stack, adding a significant dependency, or choosing between
approaches that are expensive to reverse.

## Process

1. Copy [`0000-template.md`](0000-template.md) to `NNNN-short-title.md` with the next number.
2. Keep it short: one page at most.
3. Merge the ADR (status `Accepted`) before or together with the change it describes,
   in its own PR when it changes the stack.
4. ADRs are immutable once accepted. To change a decision, write a new ADR and set the old
   one's status to `Superseded by NNNN`.

## Index

| ADR                                  | Title                       | Status   |
| ------------------------------------ | --------------------------- | -------- |
| [0001](0001-tech-stack.md)           | Technology stack            | Accepted |
| [0002](0002-linux-first.md)          | Linux as the primary target | Accepted |
| [0003](0003-license.md)              | Dual MIT / Apache-2.0       | Accepted |
| [0004](0004-yaml-parser.md)          | YAML parser (`yaml-rust2`)  | Accepted |
| [0005](0005-windows-tree-termination.md) | Windows tree termination | Accepted |
| [0006](0006-linux-woff-font-workaround.md) | Linux WOFF font workaround | Accepted |
| [0007](0007-in-app-updates.md)       | In-app updates (Tauri updater) | Accepted |
| [0008](0008-git-through-the-cli.md)  | Git through the `git` command | Accepted |
