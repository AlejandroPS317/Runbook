//! Core logic for Runbook: finding `package.json` files in a project, reading their
//! scripts and resolving pnpm/npm workspaces.
//!
//! This crate is pure Rust with no Tauri dependency, so everything here is testable
//! with `cargo test` and no window.

mod discovery;
mod manifest;
mod walk;

pub use discovery::{Discovery, DiscoveryError, Package, Problem, ProblemKind, discover};
pub use manifest::{Manifest, ManifestIssue, ParsedManifest, Script, parse_manifest};
