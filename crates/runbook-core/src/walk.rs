//! Directory traversal that finds `package.json` files.
//!
//! Uses the `ignore` crate's parallel walker. Skips dependency, VCS, cache and build-output
//! directories by name, and honors `.gitignore` files inside the project. Symlinks are not
//! followed, which also protects against link cycles.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use ignore::{DirEntry, WalkBuilder, WalkState};

const MANIFEST_FILE: &str = "package.json";

/// Directory names never searched for packages: dependencies, VCS metadata, tool caches and
/// build output. Build output often contains a copied `package.json` that is not a package
/// anyone wants to run scripts from.
const SKIPPED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".hg",
    ".svn",
    "dist",
    "build",
    "out",
    "coverage",
    "target",
    ".next",
    ".nuxt",
    ".output",
    ".svelte-kit",
    ".turbo",
    ".cache",
    ".parcel-cache",
    ".vercel",
    ".yarn",
    ".pnpm-store",
    "storybook-static",
];

/// What the walk produced: manifest paths, and traversal errors (for example a folder
/// without read permission) with the path they happened at, when known.
pub struct WalkOutput {
    pub manifests: Vec<PathBuf>,
    pub errors: Vec<(Option<PathBuf>, String)>,
}

enum Found {
    Manifest(PathBuf),
    Error(Option<PathBuf>, String),
}

/// Walks `root` and collects every `package.json` outside skipped directories.
pub fn find_manifests(root: &Path) -> WalkOutput {
    let (tx, rx) = mpsc::channel();
    builder(root).build_parallel().run(|| {
        let tx = tx.clone();
        Box::new(move |entry| {
            let found = match entry {
                Ok(entry) if is_manifest(&entry) => Some(Found::Manifest(entry.into_path())),
                Ok(_) => None,
                Err(err) => Some(Found::Error(error_path(&err), err.to_string())),
            };
            if let Some(found) = found {
                // The receiver outlives the walk, so a failed send cannot happen.
                let _ = tx.send(found);
            }
            WalkState::Continue
        })
    });
    drop(tx);

    let mut output = WalkOutput {
        manifests: Vec::new(),
        errors: Vec::new(),
    };
    for found in rx {
        match found {
            Found::Manifest(path) => output.manifests.push(path),
            Found::Error(path, message) => output.errors.push((path, message)),
        }
    }
    output
}

fn builder(root: &Path) -> WalkBuilder {
    let mut builder = WalkBuilder::new(root);
    builder
        // Start from no filters, then enable only the project's own git ignore rules:
        // hidden folders can hold real packages (e.g. `.github/actions/*`), and the user's
        // global gitignore or parent folders should not change what a project contains.
        .standard_filters(false)
        .git_ignore(true)
        .git_exclude(true)
        .follow_links(false)
        .filter_entry(|entry| !is_skipped_dir(entry));
    builder
}

fn is_manifest(entry: &DirEntry) -> bool {
    entry.file_type().is_some_and(|t| t.is_file()) && entry.file_name() == MANIFEST_FILE
}

fn is_skipped_dir(entry: &DirEntry) -> bool {
    // Depth 0 is the folder the user opened; never skip it, whatever its name.
    entry.depth() > 0
        && entry.file_type().is_some_and(|t| t.is_dir())
        && entry
            .file_name()
            .to_str()
            .is_some_and(|name| SKIPPED_DIRS.contains(&name))
}

fn error_path(err: &ignore::Error) -> Option<PathBuf> {
    match err {
        ignore::Error::WithPath { path, .. } => Some(path.clone()),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            error_path(err)
        }
        _ => None,
    }
}
