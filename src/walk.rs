//! Directory walker that finds reclaimable subtrees.
//!
//! Uses a single `WalkDir` pass with early pruning: once a reclaimable dir is
//! matched, it is not descended into (preventing double-counting of nested
//! reclaimable dirs, e.g. `node_modules` inside `target/`).

use crate::classify::{EntryKind, SurveyEntry};
use crate::size::size_dir;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// Scan a single root directory for reclaimable subtrees.
///
/// Returns all matched entries with their sizes and metadata.  The walk does
/// **not** descend into matched directories, so nested reclaimable dirs are
/// counted as part of their outermost ancestor.
///
/// # Errors
/// Returns an error if `root` cannot be read.
pub fn scan_root(root: &Path, now: DateTime<Utc>) -> Result<Vec<SurveyEntry>> {
    let mut results: Vec<SurveyEntry> = Vec::new();
    // Track which paths we've "consumed" so we can skip descending into them.
    let mut skipped: Vec<PathBuf> = Vec::new();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_under_skipped(e.path(), &skipped));

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_dir() {
            continue;
        }

        if let Some(kind) = classify_dir(&entry) {
            let path = entry.path().to_path_buf();
            let size = match size_dir(&path) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let age_days = (now - size.mtime).num_days();
            let crate_name = parent_name(&path);

            results.push(SurveyEntry {
                path: path.clone(),
                kind,
                bytes: size.bytes,
                entries: size.entries,
                mtime: size.mtime,
                age_days,
                crate_name,
            });

            // Mark this path so we don't descend into it on subsequent iterations.
            skipped.push(path);
        }
    }

    Ok(results)
}

/// Returns true if `path` is under any of the `skipped` prefixes.
fn is_under_skipped(path: &Path, skipped: &[PathBuf]) -> bool {
    skipped.iter().any(|s| path.starts_with(s) && path != s)
}

/// Classify a directory entry as a reclaimable kind, or return `None`.
fn classify_dir(entry: &DirEntry) -> Option<EntryKind> {
    let name = entry.file_name().to_string_lossy();
    let path = entry.path();

    match name.as_ref() {
        "node_modules" => Some(EntryKind::NodeModules),
        ".venv" => Some(EntryKind::Venv),
        "__pycache__" => Some(EntryKind::Pycache),
        "target" => {
            // Only match Rust build target/ dirs.
            if is_rust_target(path) {
                Some(EntryKind::RustTarget)
            } else {
                None
            }
        }
        "cache" => {
            // ~/.cargo/registry/cache or ~/.cargo/git/checkouts parent match
            // is handled separately; raw "cache" dir match is conservative.
            None
        }
        _ => {
            // Check for cargo-specific paths.
            if is_cargo_cache(path) {
                Some(EntryKind::CargoCache)
            } else {
                None
            }
        }
    }
}

/// Returns true if `path` looks like a Rust build `target/` directory.
///
/// Heuristic: parent directory contains `Cargo.toml`, OR the target dir itself
/// contains `.rustc_info.json` or `CARGO_OK`.
fn is_rust_target(path: &Path) -> bool {
    // Check for Cargo.toml in parent.
    if let Some(parent) = path.parent() {
        if parent.join("Cargo.toml").exists() {
            return true;
        }
    }
    // Check for sentinel files inside.
    path.join(".rustc_info.json").exists() || path.join("CARGO_OK").exists()
}

/// Returns true if this directory is a cargo registry cache or git checkouts dir.
fn is_cargo_cache(path: &Path) -> bool {
    // ~/.cargo/registry/cache/<registry>/
    // ~/.cargo/git/checkouts/
    let path_str = path.to_string_lossy();
    (path_str.contains("/.cargo/registry/cache") || path_str.contains("/.cargo/git/checkouts"))
        && path.is_dir()
}

/// Returns the immediate parent directory's name as the "crate" or owning dir.
fn parent_name(path: &Path) -> String {
    path.parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "<root>".to_owned())
}
