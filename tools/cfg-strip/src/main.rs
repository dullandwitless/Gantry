//! `cfg-strip`: deletes code gated behind specific Cargo features from a Rust
//! workspace, as if those features could never be enabled.
//!
//! Unlike compiling with the features off, the gated code is removed from the
//! source itself. Formatting and comments elsewhere are left untouched.
//! Anything the tool cannot handle with certainty is reported, and the exit
//! status is non-zero until the report is empty.

mod edits;
mod manifest;
mod pred;
mod rust;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Feature to treat as permanently disabled. Repeatable.
    #[arg(long = "disable", short = 'd', required = true, value_name = "FEATURE")]
    disabled: Vec<String>,

    /// Report what would change without writing anything.
    #[arg(long)]
    check: bool,

    /// Workspace root to process.
    root: PathBuf,
}

#[derive(Default)]
struct Summary {
    rewritten: Vec<PathBuf>,
    removed: Vec<PathBuf>,
    /// Formatted `path:line: severity: message` lines.
    reviews: Vec<String>,
    notes: Vec<String>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let check = args.check;
    match run(args) {
        Ok(summary) => {
            for line in summary.notes.iter().chain(&summary.reviews) {
                eprintln!("{line}");
            }
            eprintln!(
                "{} files rewritten, {} removed, {} notes, {} need review",
                summary.rewritten.len(),
                summary.removed.len(),
                summary.notes.len(),
                summary.reviews.len()
            );
            let has_changes = !summary.rewritten.is_empty() || !summary.removed.is_empty();
            if !summary.reviews.is_empty() || (check && has_changes) {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Args) -> Result<Summary> {
    let disabled: BTreeSet<String> = args.disabled.into_iter().collect();
    let files = source_files(&args.root);
    let mut summary = Summary::default();
    let mut writes: Vec<(PathBuf, String)> = Vec::new();
    let mut removals: BTreeSet<PathBuf> = BTreeSet::new();

    for path in files
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
    {
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let report = rust::strip(path, text, &disabled)?;
        for d in &report.diagnostics {
            let (label, list) = match d.severity {
                rust::Severity::Review => ("review", &mut summary.reviews),
                rust::Severity::Note => ("note", &mut summary.notes),
            };
            list.push(format!(
                "{}:{}: {label}: {}",
                path.display(),
                d.line,
                d.message
            ));
        }
        match report.outcome {
            rust::Outcome::Unchanged => {}
            rust::Outcome::Removed => {
                removals.insert(path.clone());
            }
            rust::Outcome::Rewritten {
                text,
                removed_modules,
            } => {
                removals.extend(removed_modules.into_iter().filter(|p| p.exists()));
                writes.push((path.clone(), text));
            }
        }
    }
    // A rewrite of a file that is itself being removed is moot.
    writes.retain(|(path, _)| !removals.iter().any(|r| path.starts_with(r)));

    let manifests: Vec<&PathBuf> = files.iter().filter(|p| p.ends_with("Cargo.toml")).collect();
    let mut orphaned_deps = BTreeSet::new();
    let mut used_deps = BTreeSet::new();
    let mut manifest_texts = Vec::new();
    for path in &manifests {
        let original = fs::read_to_string(path)?;
        let text = match manifest::strip(&original, &disabled)
            .with_context(|| path.display().to_string())?
        {
            Some(edit) => {
                orphaned_deps.extend(edit.removed_deps);
                writes.push(((*path).clone(), edit.text.clone()));
                edit.text
            }
            None => original,
        };
        used_deps.extend(manifest::dependency_names(&text)?);
        manifest_texts.push((*path, text));
    }
    for (path, text) in manifest_texts {
        if let Some(pruned) = manifest::prune_workspace_deps(&text, &orphaned_deps, &used_deps)? {
            writes.retain(|(p, _)| p != path);
            writes.push((path.clone(), pruned));
        }
    }

    for (path, text) in &writes {
        if !args.check {
            fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        }
        summary.rewritten.push(path.clone());
    }
    for path in &removals {
        if !args.check {
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
        summary.removed.push(path.clone());
    }
    Ok(summary)
}

fn source_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            !matches!(
                e.file_name().to_str(),
                Some("target" | ".git" | "node_modules")
            )
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs") || p.ends_with("Cargo.toml"))
        .collect()
}
