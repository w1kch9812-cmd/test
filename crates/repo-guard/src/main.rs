mod guards;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use guards::migration_version_prefixes::check_migration_version_prefixes;

fn main() {
    let exit_code = match run() {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(io::stderr(), "{error}");
            1
        }
    };
    std::process::exit(exit_code);
}

fn run() -> Result<(), String> {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "repo-guard".to_string());
    let subcommand = args
        .next()
        .ok_or_else(|| format!("usage: {program} SUBCOMMAND"))?;
    if args.next().is_some() {
        return Err(format!("usage: {program} SUBCOMMAND"));
    }

    match subcommand.as_str() {
        "migration-version-prefixes" => {
            let repo_root = resolve_repo_root()?;
            let report = check_migration_version_prefixes(&repo_root)?;
            writeln!(
                io::stdout(),
                "migration-version-prefixes-ok files={}",
                report.files
            )
            .map_err(|error| format!("failed to write guard output: {error}"))?;
            Ok(())
        }
        _ => Err(format!("unknown subcommand: {subcommand}")),
    }
}

fn resolve_repo_root() -> Result<PathBuf, String> {
    let current_dir = env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?;
    let mut candidate = Some(current_dir.as_path());
    let mut first_manifest_dir = None;

    while let Some(dir) = candidate {
        if dir.join(".git").exists() || has_workspace_manifest(dir) {
            return Ok(dir.to_path_buf());
        }
        if first_manifest_dir.is_none() && dir.join("Cargo.toml").is_file() {
            first_manifest_dir = Some(dir.to_path_buf());
        }
        candidate = dir.parent();
    }

    first_manifest_dir.ok_or_else(|| "repo root is missing: Cargo.toml or .git".to_string())
}

fn has_workspace_manifest(dir: &Path) -> bool {
    let manifest = dir.join("Cargo.toml");
    fs::read_to_string(manifest)
        .is_ok_and(|content| content.lines().any(|line| line == "[workspace]"))
}
