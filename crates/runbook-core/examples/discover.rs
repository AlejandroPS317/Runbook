//! Prints what Runbook discovers in a folder, and how long it took.
//!
//! Usage: `cargo run -p runbook-core --example discover -- <folder>`

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use runbook_core::PackageRole;

fn main() -> ExitCode {
    let Some(root) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: discover <folder>");
        return ExitCode::FAILURE;
    };

    let started = Instant::now();
    let discovery = match runbook_core::discover(&root) {
        Ok(discovery) => discovery,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };
    let elapsed = started.elapsed();

    println!("package manager: {}", discovery.package_manager);
    match &discovery.workspaces {
        Some(workspaces) => println!(
            "workspaces ({:?}): {}",
            workspaces.source,
            workspaces.patterns.join(", ")
        ),
        None => println!("workspaces: none"),
    }
    for package in &discovery.packages {
        let dir = if package.relative_dir.is_empty() {
            "."
        } else {
            &package.relative_dir
        };
        let name = package.name.as_deref().unwrap_or("(no name)");
        let role = match package.role {
            PackageRole::Root => "root",
            PackageRole::Member => "member",
            PackageRole::Unlisted => "unlisted",
        };
        println!("{dir}  {name}  [{role}]");
        for script in &package.scripts {
            println!("    {:<20} {}", script.name, script.command);
        }
    }
    for problem in &discovery.problems {
        println!("problem: {}: {}", problem.relative_path, problem.kind);
    }
    let script_count: usize = discovery.packages.iter().map(|p| p.scripts.len()).sum();
    println!(
        "{} packages, {} scripts, {} problems in {:.1} ms",
        discovery.packages.len(),
        script_count,
        discovery.problems.len(),
        elapsed.as_secs_f64() * 1000.0
    );
    ExitCode::SUCCESS
}
