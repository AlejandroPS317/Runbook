// Writes the Tauri updater manifest (`latest.json`, ADR 0007) for a release to stdout.
//
// Usage: node updater-manifest.mjs <artifacts dir> <tag> <owner/repo>
//
// Every update bundle must sit next to the `.sig` file the Tauri CLI writes when
// `createUpdaterArtifacts` is on. Keys are `{os}-{arch}-{bundle type}`, the updater's first
// lookup; `darwin-aarch64` and `windows-x86_64` are its fallback for installs without a
// bundle type. Architectures match the Release workflow's runners: x86_64 Linux and
// Windows, Apple Silicon macOS.

import { readdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";

const BUNDLES = [
  { suffix: ".AppImage", keys: ["linux-x86_64-appimage"] },
  { suffix: ".deb", keys: ["linux-x86_64-deb"] },
  { suffix: ".app.tar.gz", keys: ["darwin-aarch64-app", "darwin-aarch64"] },
  { suffix: "-setup.exe", keys: ["windows-x86_64-nsis", "windows-x86_64"] },
  { suffix: ".msi", keys: ["windows-x86_64-msi"] },
];

const [dir, tag, repo] = process.argv.slice(2);
if (!dir || !tag || !repo) {
  console.error("usage: node updater-manifest.mjs <artifacts dir> <tag> <owner/repo>");
  process.exit(2);
}

const files = readdirSync(dir, { recursive: true, withFileTypes: true })
  .filter((entry) => entry.isFile())
  .map((entry) => join(entry.parentPath, entry.name));

const platforms = {};
for (const { suffix, keys } of BUNDLES) {
  const matches = files.filter((file) => file.endsWith(suffix));
  if (matches.length !== 1) {
    console.error(`expected one *${suffix} update bundle, found ${matches.length}`);
    process.exit(1);
  }
  const [file] = matches;
  if (!files.includes(`${file}.sig`)) {
    console.error(`${file} has no .sig: is TAURI_SIGNING_PRIVATE_KEY set?`);
    process.exit(1);
  }
  const entry = {
    // The `.sig` content itself; the updater does not accept a path or URL here.
    signature: readFileSync(`${file}.sig`, "utf8").trim(),
    url: `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(basename(file))}`,
  };
  for (const key of keys) platforms[key] = entry;
}

const manifest = {
  version: tag.replace(/^v/, ""),
  notes: `https://github.com/${repo}/releases/tag/${tag}`,
  pub_date: new Date().toISOString(),
  platforms,
};
process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
