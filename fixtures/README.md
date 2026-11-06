# Fixtures

Sample projects used by tests. Each folder is opened as a project root.

- `pnpm-turbo-monorepo/`: mirrors the user's real pnpm + turbo monorepo (root scripts
  calling `turbo run`, pnpm `--filter` scripts, packages in `apps/` and `packages/`, one
  package without scripts). Contains decoy `package.json` files in `node_modules/`, `dist/`
  and `.turbo/` that discovery must skip.
  `pnpm-workspace.yaml` mirrors the real file too (comments, `catalog`, `catalogs`) and
  excludes `packages/legacy` with a negated pattern; `examples/demo` is outside the
  workspace.
- `npm-workspaces/`: npm `workspaces` in the root `package.json`, including a member in
  `tools/build` (a folder name discovery normally skips) and an unlisted `docs/site`.
- `edge-cases/`: one malformed or unusual `package.json` per folder, named after the case.

The root `.gitignore` re-includes `node_modules/` and `dist/` under `fixtures/`. Tests that
need a `.git` folder or `.gitignore` behavior build their project in a temporary directory.
